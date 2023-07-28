use super::guest_funcs::CallGuestFunction;
use super::mem_mgr::MemMgr;
use super::{
    host_funcs::default_writer_func, host_funcs::HostFuncs, host_funcs::HostFunctionsMap,
    initialized::Sandbox,
};
use super::{host_funcs::CallHostPrint, run_options::SandboxRunOptions};
use crate::func::host::function_definition::HostFunctionDefinition;
use crate::func::host::{HostFunction1, HyperlightFunction};
use crate::hypervisor::Hypervisor;
use crate::mem::mgr::STACK_COOKIE_LEN;
use crate::mem::ptr::RawPtr;
use crate::mem::{
    config::SandboxMemoryConfiguration, layout::SandboxMemoryLayout, mgr::SandboxMemoryManager,
    pe::pe_info::PEInfo,
};
use crate::sandbox_state::transition::Noop;
use crate::sandbox_state::{sandbox::EvolvableSandbox, transition::MutatingCallback};
#[cfg(target_os = "linux")]
use crate::{
    hypervisor::hyperv_linux::{self, HypervLinuxDriver},
    hypervisor::hypervisor_mem::HypervisorAddrs,
    hypervisor::kvm,
    hypervisor::kvm::KVMDriver,
    mem::ptr::GuestPtr,
    mem::ptr_offset::Offset,
};
use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use std::ffi::c_void;
use std::ops::Add;
use std::option::Option;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tracing::instrument;

/// A preliminary `Sandbox`, not yet ready to execute guest code.
///
/// Prior to initializing a full-fledged `Sandbox`, you must create one of
/// these `UninitializedSandbox`es with the `new` function, register all the
/// host-implemented functions you need to be available to the guest, then
/// call either `initialize` or `evolve to transform your
/// `UninitializedSandbox` into an initialized `Sandbox`.
#[allow(unused)]
pub struct UninitializedSandbox<'a> {
    // Registered host functions
    host_functions: HostFunctionsMap<'a>,
    // The memory manager for the sandbox.
    mem_mgr: SandboxMemoryManager,
    stack_guard: [u8; STACK_COOKIE_LEN],
    executing_guest_call: AtomicBool,
    needs_state_reset: bool,
    // ^^^ `UninitializedSandbox` should
    // also cointain `executing_guest_call`,
    // and `needs_state_reset` because it might
    // execute some guest functions when initializing.
}

impl<'a> std::fmt::Debug for UninitializedSandbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UninitializedSandbox")
            .field("stack_guard", &self.stack_guard)
            .field("num_host_funcs", &self.host_functions.len())
            .finish()
    }
}

impl<'a> crate::sandbox_state::sandbox::Sandbox for UninitializedSandbox<'a> {}

impl<'a, F>
    EvolvableSandbox<
        UninitializedSandbox<'a>,
        Sandbox<'a>,
        MutatingCallback<'a, UninitializedSandbox<'a>, F>,
    > for UninitializedSandbox<'a>
where
    F: FnOnce(&mut UninitializedSandbox<'a>) -> Result<()> + 'a,
{
    /// Evolve `self` into a `Sandbox`, executing a caller-provided
    /// callback during the transition process.
    ///
    /// If you need to do this transition without a callback, use the
    /// `EvolvableSandbox` implementation that takes a `Noop`.
    fn evolve(mut self, tsn: MutatingCallback<UninitializedSandbox<'a>, F>) -> Result<Sandbox<'a>> {
        tsn.call(&mut self)?;
        // TODO: snapshot memory here so we can take the returned
        // Sandbox and revert back to an UninitializedSandbox
        Ok(Sandbox::from(self))
    }
}

impl<'a>
    EvolvableSandbox<
        UninitializedSandbox<'a>,
        Sandbox<'a>,
        Noop<UninitializedSandbox<'a>, Sandbox<'a>>,
    > for UninitializedSandbox<'a>
{
    /// Evolve `self` to a `Sandbox` without any additional metadata.
    ///
    /// If you want to pass a callback to this state transition so you can
    /// run your own code during the transition, use the `EvolvableSandbox`
    /// implementation that accepts a `MutatingCallback`
    fn evolve(self, _: Noop<UninitializedSandbox<'a>, Sandbox<'a>>) -> Result<Sandbox<'a>> {
        // TODO: snapshot memory here so we can take the returned
        // Sandbox and revert back to an UninitializedSandbox
        Ok(Sandbox::from(self))
    }
}

impl<'a> HostFuncs<'a> for UninitializedSandbox<'a> {
    fn get_host_funcs(&self) -> &HostFunctionsMap<'a> {
        &self.host_functions
    }
}

impl<'a> CallHostPrint<'a> for UninitializedSandbox<'a> {}

impl<'a> CallGuestFunction<'a> for UninitializedSandbox<'a> {}

impl<'a> MemMgr for UninitializedSandbox<'a> {
    fn get_mem_mgr(&self) -> &SandboxMemoryManager {
        &self.mem_mgr
    }

    fn get_mem_mgr_mut(&mut self) -> &mut SandboxMemoryManager {
        &mut self.mem_mgr
    }

    fn get_stack_cookie(&self) -> &super::mem_mgr::StackCookie {
        &self.stack_guard
    }
}

impl<'a> UninitializedSandbox<'a> {
    /// Create a new sandbox configured to run the binary at path
    /// `bin_path`.
    ///
    /// The instrument attribute is used to generate tracing spans and also to emit an error should the Result be an error
    /// In order to ensure that the span is associated with any error (so we get the correlation id ) we set the level of the span to error
    /// the downside to this is that if there is no trace subscriber a log at level error or below will always emit a record for this regardless of if an error actually occurs
    #[instrument(err(), skip(cfg), name = "UninitializedSandbox::new")]
    pub fn new(
        bin_path: String,
        cfg: Option<SandboxMemoryConfiguration>,
        sandbox_run_options: Option<SandboxRunOptions>,
    ) -> Result<Self> {
        // Make sure the binary exists

        let path = Path::new(&bin_path)
            .canonicalize()
            .map_err(|e| anyhow!("Error {} File Path {}", e, &bin_path))?;
        path.try_exists()
            .map_err(|e| anyhow!("Error {} File Path {}", e, &bin_path))?;

        let sandbox_run_options =
            sandbox_run_options.unwrap_or(SandboxRunOptions::RUN_IN_HYPERVISOR);

        let run_from_process_memory = sandbox_run_options
            .contains(SandboxRunOptions::RUN_IN_PROCESS)
            || sandbox_run_options.contains(SandboxRunOptions::RUN_FROM_GUEST_BINARY);
        let run_from_guest_binary =
            sandbox_run_options.contains(SandboxRunOptions::RUN_FROM_GUEST_BINARY);

        if run_from_guest_binary
            && sandbox_run_options.contains(SandboxRunOptions::RECYCLE_AFTER_RUN)
        {
            anyhow::bail!("Recycle after run at is not supported when running from guest binary.");
        }

        let mem_cfg = cfg.unwrap_or_default();
        let mut mem_mgr = UninitializedSandbox::load_guest_binary(
            mem_cfg,
            &bin_path,
            run_from_process_memory,
            run_from_guest_binary,
        )?;

        // <WriteMemoryLayout>
        let layout = mem_mgr.layout;
        let shared_mem = mem_mgr.get_shared_mem_mut();
        let mem_size = shared_mem.mem_size();
        let guest_offset = if run_from_process_memory {
            shared_mem.base_addr()
        } else {
            SandboxMemoryLayout::BASE_ADDRESS
        };
        layout.write(shared_mem, guest_offset, mem_size)?;
        // </WriteMemoryLayout>

        let stack_guard = Self::create_stack_guard();
        mem_mgr.set_stack_guard(&stack_guard)?;

        // The default writer function is to write to stdout with green text.
        let default_writer = Arc::new(Mutex::new(default_writer_func));

        let mut sandbox = Self {
            host_functions: HashMap::new(),
            mem_mgr,
            stack_guard,
            executing_guest_call: AtomicBool::new(false),
            needs_state_reset: false,
        };

        default_writer.register(&mut sandbox, "writer_func")?;

        Ok(sandbox)
    }

    fn create_stack_guard() -> [u8; STACK_COOKIE_LEN] {
        rand::random::<[u8; STACK_COOKIE_LEN]>()
    }

    /// Register a host function with the sandbox.
    pub(crate) fn register_host_function(
        &mut self,
        hfd: &HostFunctionDefinition,
        func: HyperlightFunction<'a>,
    ) -> Result<()> {
        self.host_functions
            .insert(hfd.function_name.to_string(), func);
        let buffer: Vec<u8> = hfd.try_into()?;
        self.mem_mgr.write_host_function_definition(&buffer)?;
        Ok(())
    }

    /// Set up the appropriate hypervisor for the platform.
    ///
    /// this function is used to prevent clippy from complaining
    /// the 'mgr' param is unused on windows builds. this function and the
    /// function of the same name on linux builds will merge when we
    /// have a complete WHP implementation in Rust.
    ///
    /// TODO: remove this dead_code annotation after it's hooked up in
    /// https://github.com/deislabs/hyperlight/pull/727/files, and merge with
    /// linux version of this function
    #[allow(dead_code)]
    #[cfg(target_os = "windows")]
    fn set_up_hypervisor_partition(_: &mut SandboxMemoryManager) -> Result<Box<dyn Hypervisor>> {
        bail!("Hyperlight does not yet support Windows");
    }

    /// Set up the appropriate hypervisor for the platform
    ///
    /// TODO: remove this dead_code annotation after it's hooked up in
    /// https://github.com/deislabs/hyperlight/pull/727/files,
    /// and merge with the windows version of this function
    #[allow(dead_code)]
    #[cfg(target_os = "linux")]
    fn set_up_hypervisor_partition(mgr: &mut SandboxMemoryManager) -> Result<Box<dyn Hypervisor>> {
        let mem_size = u64::try_from(mgr.shared_mem.mem_size())?;
        let rsp = mgr.set_up_hypervisor_partition(mem_size)?;
        let base_addr = SandboxMemoryLayout::BASE_ADDRESS;
        let pml4_addr = base_addr + SandboxMemoryLayout::PML4_OFFSET;
        let entrypoint = {
            let load_addr = mgr.load_addr.clone();
            let load_offset_u64 =
                u64::from(load_addr) - u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?;
            let total_offset = Offset::from(load_offset_u64) + mgr.entrypoint_offset;
            GuestPtr::try_from(total_offset)
        }?;
        if hyperv_linux::is_hypervisor_present()? {
            let guest_pfn = u64::try_from(SandboxMemoryLayout::BASE_ADDRESS >> 12)?;
            let host_addr = u64::try_from(mgr.shared_mem.base_addr())?;
            let addrs = HypervisorAddrs {
                entrypoint: entrypoint.absolute()?,
                guest_pfn,
                host_addr,
                mem_size,
            };
            let hv = HypervLinuxDriver::new(&addrs)?;
            Ok(Box::new(hv))
        } else if kvm::is_hypervisor_present().is_ok() {
            let host_addr = u64::try_from(mgr.shared_mem.base_addr())?;
            let hv = KVMDriver::new(
                host_addr,
                u64::try_from(pml4_addr)?,
                mem_size,
                entrypoint.absolute()?,
                rsp,
            )?;
            Ok(Box::new(hv))
        } else {
            bail!("Linux platform detected, but neither KVM nor Linux HyperV detected")
        }
    }

    /// Call the entry point inside this `Sandbox`
    pub(crate) unsafe fn call_entry_point(
        &self,
        peb_address: RawPtr,
        seed: u64,
        page_size: u32,
    ) -> Result<()> {
        type EntryPoint = extern "C" fn(i64, u64, u32) -> i32;
        let entry_point: EntryPoint = {
            let addr = {
                let offset = self.mem_mgr.entrypoint_offset;
                self.mem_mgr.load_addr.clone().add(offset)
            };

            let fn_location = u64::from(addr) as *const c_void;
            std::mem::transmute(fn_location)
        };
        let peb_i64 = i64::try_from(u64::from(peb_address))?;
        entry_point(peb_i64, seed, page_size);
        Ok(())
    }

    /// Load the file at `bin_path_str` into a PE file, then attempt to
    /// load the PE file into a `SandboxMemoryManager` and return it.
    ///
    /// If `run_from_guest_binary` is passed as `true`, and this code is
    /// running on windows, this function will call
    /// `SandboxMemoryManager::load_guest_binary_using_load_library` to
    /// create the new `SandboxMemoryManager`. If `run_from_guest_binary` is
    /// passed as `true` and we're not running on windows, this function will
    /// return an `Err`. Otherwise, if `run_from_guest_binary` is passed
    /// as `false`, this function calls `SandboxMemoryManager::load_guest_binary_into_memory`.
    pub(super) fn load_guest_binary(
        mem_cfg: SandboxMemoryConfiguration,
        bin_path_str: &str,
        run_from_process_memory: bool,
        run_from_guest_binary: bool,
    ) -> Result<SandboxMemoryManager> {
        let mut pe_info = PEInfo::from_file(bin_path_str)?;
        if run_from_guest_binary {
            SandboxMemoryManager::load_guest_binary_using_load_library(
                mem_cfg,
                bin_path_str,
                &mut pe_info,
                run_from_process_memory,
            )
            // TODO: This produces the wrong error message on Linux and is possibly obsfucating the real error on Windows
            .map_err(|_| {
                let err_msg =
                    "Only one instance of Sandbox is allowed when running from guest binary";
                anyhow!(err_msg)
            })
        } else {
            SandboxMemoryManager::load_guest_binary_into_memory(
                mem_cfg,
                &mut pe_info,
                run_from_process_memory,
            )
        }
    }

    /// Initialize the `Sandbox` from an `UninitializedSandbox`.
    /// Receives a callback function to be called during initialization.
    #[allow(unused)]
    #[instrument(err(Debug), skip_all)]
    pub(super) fn initialize<F: Fn(&mut UninitializedSandbox<'a>) -> Result<()> + 'a>(
        mut self,
        callback: Option<F>,
    ) -> Result<Sandbox<'a>> {
        match callback {
            Some(cb) => self.evolve(MutatingCallback::from(cb)),
            None => self.evolve(Noop::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sandbox::mem_mgr::MemMgr;
    use crate::testing::{
        log_values::test_value_as_str, logger::Logger as TestLogger, logger::LOGGER as TEST_LOGGER,
        tracing_subscriber::TracingSubscriber as TestSubcriber,
    };
    use crate::{
        func::{
            host::{HostFunction1, HostFunction2},
            types::{ParameterValue, ReturnValue},
        },
        mem::config::SandboxMemoryConfiguration,
        sandbox::host_funcs::CallHostPrint,
        testing::simple_guest_path,
        Sandbox, SandboxRunOptions, UninitializedSandbox,
    };
    use crate::{sandbox::host_funcs::CallHostFunction, testing::log_values::try_to_strings};
    use anyhow::Result;
    use crossbeam_queue::ArrayQueue;
    use log::Level;
    use serde_json::{Map, Value};
    use serial_test::serial;
    use std::{
        io::{Read, Write},
        sync::{Arc, Mutex},
    };
    use std::{path::PathBuf, thread};
    use tempfile::NamedTempFile;
    use tracing::Level as tracing_level;
    use tracing_core::{callsite::rebuild_interest_cache, Subscriber};
    use uuid::Uuid;

    #[test]
    fn test_new_sandbox() {
        // Guest Binary exists at path

        let binary_path = simple_guest_path().unwrap();
        let sandbox = UninitializedSandbox::new(binary_path.clone(), None, None);
        assert!(sandbox.is_ok());

        // Guest Binary does not exist at path

        let binary_path_does_not_exist = binary_path.trim_end_matches(".exe").to_string();
        let uninitialized_sandbox =
            UninitializedSandbox::new(binary_path_does_not_exist, None, None);
        assert!(uninitialized_sandbox.is_err());

        // Non default memory configuration

        let cfg = SandboxMemoryConfiguration::new(
            0x1000,
            0x1000,
            0x1000,
            0x1000,
            0x1000,
            Some(0x1000),
            Some(0x1000),
        );

        let uninitialized_sandbox = UninitializedSandbox::new(binary_path.clone(), Some(cfg), None);
        assert!(uninitialized_sandbox.is_ok());

        // Invalid sandbox_run_options

        let sandbox_run_options =
            SandboxRunOptions::RUN_FROM_GUEST_BINARY | SandboxRunOptions::RECYCLE_AFTER_RUN;

        let uninitialized_sandbox =
            UninitializedSandbox::new(binary_path.clone(), None, Some(sandbox_run_options));
        assert!(uninitialized_sandbox.is_err());

        let uninitialized_sandbox = UninitializedSandbox::new(binary_path, None, None);
        assert!(uninitialized_sandbox.is_ok());

        // Get a Sandbox from an uninitialized sandbox without a call back function

        let sandbox = uninitialized_sandbox
            .unwrap()
            .initialize::<fn(&mut UninitializedSandbox<'_>) -> Result<()>>(None);
        assert!(sandbox.is_ok());

        // Test with  init callback function
        // TODO: replace this with a test that registers and calls functions once we have that functionality

        let mut received_msg = String::new();

        let writer = |msg| {
            received_msg = msg;
            Ok(())
        };

        let writer_func = Arc::new(Mutex::new(writer));

        let mut uninitialized_sandbox = UninitializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            None,
        )
        .expect("Failed to create sandbox");

        writer_func
            .register(&mut uninitialized_sandbox, "writer_func")
            .expect("Failed to register writer function");

        fn init(uninitialized_sandbox: &mut UninitializedSandbox) -> Result<()> {
            uninitialized_sandbox.host_print("test".to_string())
        }

        let sandbox = uninitialized_sandbox.initialize(Some(init));
        assert!(sandbox.is_ok());

        drop(sandbox);

        assert_eq!(&received_msg, "test");
    }

    #[test]
    fn test_load_guest_binary_manual() {
        let cfg = SandboxMemoryConfiguration::default();

        let simple_guest_path = simple_guest_path().unwrap();
        let mgr =
            UninitializedSandbox::load_guest_binary(cfg, simple_guest_path.as_str(), false, false)
                .unwrap();
        assert_eq!(cfg, mgr.mem_cfg);
    }

    #[test]
    fn test_stack_guard() {
        let simple_guest_path = simple_guest_path().unwrap();
        let sbox = UninitializedSandbox::new(simple_guest_path, None, None).unwrap();
        let res = sbox.check_stack_guard();
        assert!(
            res.is_ok(),
            "UninitializedSandbox::check_stack_guard returned an error"
        );
        assert!(
            res.unwrap(),
            "UninitializedSandbox::check_stack_guard returned false"
        );
    }

    #[test]
    fn test_host_functions() {
        let uninitialized_sandbox = || {
            UninitializedSandbox::new(
                simple_guest_path().expect("Guest Binary Missing"),
                None,
                None,
            )
            .unwrap()
        };
        fn init(_: &mut UninitializedSandbox) -> Result<()> {
            Ok(())
        }

        // simple register + call
        {
            let mut usbox = uninitialized_sandbox();
            let test0 = |arg: i32| -> Result<i32> { Ok(arg + 1) };
            let test_func0 = Arc::new(Mutex::new(test0));
            test_func0.register(&mut usbox, "test0").unwrap();

            let sandbox = usbox.initialize(Some(init));
            assert!(sandbox.is_ok());
            let mut sandbox = sandbox.unwrap();

            let res = sandbox
                .call_host_function("test0", vec![ParameterValue::Int(1)])
                .unwrap();

            assert_eq!(res, ReturnValue::Int(2));
        }

        // multiple parameters register + call
        {
            let mut usbox = uninitialized_sandbox();
            let test1 = |arg1: i32, arg2: i32| -> Result<i32> { Ok(arg1 + arg2) };
            let test_func1 = Arc::new(Mutex::new(test1));
            test_func1.register(&mut usbox, "test1").unwrap();

            let sandbox = usbox.initialize(Some(init));
            assert!(sandbox.is_ok());
            let mut sandbox = sandbox.unwrap();

            let res = sandbox
                .call_host_function(
                    "test1",
                    vec![ParameterValue::Int(1), ParameterValue::Int(2)],
                )
                .unwrap();

            assert_eq!(res, ReturnValue::Int(3));
        }

        // incorrect arguments register + call
        {
            let mut usbox = uninitialized_sandbox();
            let test2 = |arg1: String| -> Result<()> {
                println!("test2 called: {}", arg1);
                Ok(())
            };
            let test_func2 = Arc::new(Mutex::new(test2));
            test_func2.register(&mut usbox, "test2").unwrap();

            let sandbox = usbox.initialize(Some(init));
            assert!(sandbox.is_ok());
            let mut sandbox = sandbox.unwrap();

            let res = sandbox.call_host_function("test2", vec![]);
            assert!(res.is_err());
        }

        // calling a function that doesn't exist
        {
            let usbox = uninitialized_sandbox();
            let sandbox = usbox.initialize(Some(init));
            assert!(sandbox.is_ok());
            let mut sandbox = sandbox.unwrap();

            let res = sandbox.call_host_function("test4", vec![]);
            assert!(res.is_err());
        }
    }

    #[test]
    #[serial]
    fn test_load_guest_binary_load_lib() {
        let cfg = SandboxMemoryConfiguration::default();
        let simple_guest_path = simple_guest_path().unwrap();
        let mgr_res =
            UninitializedSandbox::load_guest_binary(cfg, simple_guest_path.as_str(), true, true);
        #[cfg(target_os = "linux")]
        {
            assert!(mgr_res.is_err())
        }
        #[cfg(target_os = "windows")]
        {
            let _ = mgr_res.unwrap();
        }
    }

    #[test]
    fn test_host_print() {
        // writer as a FnMut closure mutating a captured variable and then trying to access the captured variable
        // after the Sandbox instance has been dropped
        // this example is fairly contrived but we should still support such an approach.

        let mut received_msg = String::new();

        let writer = |msg| {
            received_msg = msg;
            Ok(())
        };

        let writer_func = Arc::new(Mutex::new(writer));

        let mut sandbox = UninitializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            None,
        )
        .expect("Failed to create sandbox");

        writer_func
            .register(&mut sandbox, "writer_func")
            .expect("Failed to register writer function");

        sandbox.host_print("test".to_string()).unwrap();

        drop(sandbox);

        assert_eq!(&received_msg, "test");

        // There may be cases where a mutable reference to the captured variable is not required to be used outside the closue
        // e.g. if the function is writing to a file or a socket etc.

        // writer as a FnMut closure mutating a captured variable but not trying to access the captured variable

        // This seems more realistic as the client is creating a file to be written to in the closure
        // and then accessing the file a different handle.
        // The problem is that captured_file still needs static lifetime so even though we can access the data through the second file handle
        // this still does not work as the captured_file is dropped at the end of the function

        let mut captured_file = NamedTempFile::new().unwrap();
        let mut file = captured_file.reopen().unwrap();

        let writer = |msg: String| -> Result<()> {
            captured_file.write_all(msg.as_bytes()).unwrap();
            Ok(())
        };

        let writer_func = Arc::new(Mutex::new(writer));

        let mut sandbox = UninitializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            None,
        )
        .expect("Failed to create sandbox");

        writer_func
            .register(&mut sandbox, "writer_func")
            .expect("Failed to register writer function");

        sandbox.host_print("test2".to_string()).unwrap();

        let mut buffer = String::new();
        file.read_to_string(&mut buffer).unwrap();
        assert_eq!(buffer, "test2");

        // writer as a function

        fn fn_writer(msg: String) -> Result<()> {
            assert_eq!(msg, "test2");
            Ok(())
        }

        let writer_func = Arc::new(Mutex::new(fn_writer));
        let mut sandbox = UninitializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            None,
        )
        .expect("Failed to create sandbox");

        writer_func
            .register(&mut sandbox, "writer_func")
            .expect("Failed to register writer function");

        sandbox.host_print("test2".to_string()).unwrap();

        // writer as a method

        let mut test_host_print = TestHostPrint::new();

        // create a closure over the struct method

        let writer_closure = |s| test_host_print.write(s);

        let writer_method = Arc::new(Mutex::new(writer_closure));

        let mut sandbox = UninitializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            None,
        )
        .expect("Failed to create sandbox");

        writer_method
            .register(&mut sandbox, "writer_func")
            .expect("Failed to register writer function");

        sandbox.host_print("test3".to_string()).unwrap();
    }

    struct TestHostPrint {}

    impl TestHostPrint {
        fn new() -> Self {
            TestHostPrint {}
        }

        fn write(&mut self, msg: String) -> Result<()> {
            assert_eq!(msg, "test3");
            Ok(())
        }
    }

    #[test]
    fn check_create_and_use_sandbox_on_different_threads() {
        let unintializedsandbox_queue = Arc::new(ArrayQueue::<UninitializedSandbox>::new(10));
        let sandbox_queue = Arc::new(ArrayQueue::<Sandbox>::new(10));

        for i in 0..10 {
            let simple_guest_path = simple_guest_path().expect("Guest Binary Missing");
            let unintializedsandbox = UninitializedSandbox::new(simple_guest_path, None, None)
                .unwrap_or_else(|_| panic!("Failed to create UninitializedSandbox {}", i));

            unintializedsandbox_queue
                .push(unintializedsandbox)
                .unwrap_or_else(|_| panic!("Failed to push UninitializedSandbox {}", i));
        }

        let thread_handles = (0..10)
            .map(|i| {
                let uq = unintializedsandbox_queue.clone();
                let sq = sandbox_queue.clone();
                thread::spawn(move || {
                    let mut uninitialized_sandbox = uq.pop().unwrap_or_else(|| {
                        panic!("Failed to pop UninitializedSandbox thread {}", i)
                    });
                    uninitialized_sandbox
                        .host_print(format!("Print from UninitializedSandbox on Thread {}\n", i))
                        .unwrap();

                    let sandbox = uninitialized_sandbox
                        .initialize::<fn(&mut UninitializedSandbox<'_>) -> Result<()>>(None)
                        .unwrap_or_else(|_| {
                            panic!("Failed to initialize UninitializedSandbox thread {}", i)
                        });

                    sq.push(sandbox).unwrap_or_else(|_| {
                        panic!("Failed to push UninitializedSandbox thread {}", i)
                    })
                })
            })
            .collect::<Vec<_>>();

        for handle in thread_handles {
            handle.join().unwrap();
        }

        let thread_handles = (0..10)
            .map(|i| {
                let sq = sandbox_queue.clone();
                thread::spawn(move || {
                    let mut sandbox = sq
                        .pop()
                        .unwrap_or_else(|| panic!("Failed to pop Sandbox thread {}", i));
                    sandbox
                        .host_print(format!("Print from Sandbox on Thread {}\n", i))
                        .unwrap();
                })
            })
            .collect::<Vec<_>>();

        for handle in thread_handles {
            handle.join().unwrap();
        }
    }

    #[test]
    #[ignore]
    // Tests that trace data are emitted when a trace subscriber is set
    // this test is ignored because it is incompatible with other tests , specifically those which require a logger for tracing
    // to run tracing tests use `cargo test test_trace -- --ignored`
    fn test_trace_trace() {
        TestLogger::initialize_log_tracer();
        rebuild_interest_cache();
        let subscriber = TestSubcriber::new(tracing_level::TRACE);
        tracing::subscriber::with_default(subscriber.clone(), || {
            let correlation_id = Uuid::new_v4().as_hyphenated().to_string();
            let span = tracing::error_span!("test_trace_logs", correlation_id).entered();

            // We should be in span 1

            let current_span = subscriber.current_span();
            assert!(current_span.is_known(), "Current span is unknown");
            let current_span_metadata = current_span.into_inner().unwrap();
            assert_eq!(
                current_span_metadata.0.into_u64(),
                1,
                "Current span is not span 1"
            );
            assert_eq!(current_span_metadata.1.name(), "test_trace_logs");

            // Get the span data and check the correlation id

            let span_data = subscriber.get_span(1);
            let span_attributes: &Map<String, Value> = span_data
                .get("span")
                .unwrap()
                .get("attributes")
                .unwrap()
                .as_object()
                .unwrap();

            test_value_as_str(span_attributes, "correlation_id", correlation_id.as_str());

            let mut binary_path = simple_guest_path().unwrap();
            binary_path.push_str("does_not_exist");

            let sbox = UninitializedSandbox::new(binary_path, None, None);
            assert!(sbox.is_err());

            // Now we should still be in span 1 but span 2 should be created (we created entered and exited span 2 when we called UninitializedSandbox::new)

            let current_span = subscriber.current_span();
            assert!(current_span.is_known(), "Current span is unknown");
            let current_span_metadata = current_span.into_inner().unwrap();
            assert_eq!(
                current_span_metadata.0.into_u64(),
                1,
                "Current span is not span 1"
            );

            let span_metadata = subscriber.get_span_metadata(2);
            assert_eq!(span_metadata.name(), "UninitializedSandbox::new");

            // There should be one event for the error that the binary path does not exist

            let events = subscriber.get_events();
            assert_eq!(events.len(), 1);

            let mut count_matching_events = 0;

            for json_value in events {
                let event_values = json_value.as_object().unwrap().get("event").unwrap();
                let metadata_values_map =
                    event_values.get("metadata").unwrap().as_object().unwrap();
                let event_values_map = event_values.as_object().unwrap();

                #[cfg(target_os = "windows")]
                let expected_error =
                    "Error The system cannot find the file specified. (os error 2) File Path";
                #[cfg(not(target_os = "windows"))]
                let expected_error = "Error No such file or directory (os error 2) File Path";

                let err_vals_res = try_to_strings([
                    (metadata_values_map, "level"),
                    (event_values_map, "error"),
                    (metadata_values_map, "module_path"),
                    (metadata_values_map, "target"),
                ]);
                if let Ok(err_vals) = err_vals_res {
                    if err_vals[0] == "ERROR"
                        && err_vals[1].starts_with(expected_error)
                        && err_vals[2] == "hyperlight_host::sandbox::uninitialized"
                        && err_vals[3] == "hyperlight_host::sandbox::uninitialized"
                    {
                        count_matching_events += 1;
                    }
                }
            }
            assert!(
                count_matching_events == 1,
                "Unexpected number of matching events {}",
                count_matching_events
            );
            span.exit();
            subscriber.clear();
        });
    }

    #[test]
    // Tests that traces are emitted as log records when there is no trace
    // subscriber configured.
    fn test_log_trace() {
        {
            TestLogger::initialize_test_logger();
            TEST_LOGGER.set_max_level(log::LevelFilter::Trace);

            // This makes sure that the metadata interest cache is rebuilt so that
            // the log records are emitted for the trace records

            rebuild_interest_cache();

            let mut invalid_binary_path = simple_guest_path().unwrap();
            invalid_binary_path.push_str("does_not_exist");

            let sbox = UninitializedSandbox::new(invalid_binary_path, None, None);
            assert!(sbox.is_err());

            // When tracing is creating log records it will create a log
            // record for the creation of the span (from the instrument
            // attribute), and will then create a log record for the entry to
            // and exit from the span.
            //
            // It also creates a log record for the span being dropped.
            // So we expect 5 log records for this test, four for the span and
            // then one for the error as the file that we are attempting to
            // load into the sandbox does not exist

            let num_calls = TEST_LOGGER.num_log_calls();
            assert_eq!(5, num_calls);

            // Log record 1

            let logcall = TEST_LOGGER.get_log_call(0).unwrap();
            assert_eq!(Level::Info, logcall.level);

            assert!(logcall
                .args
                .starts_with("UninitializedSandbox::new; bin_path"));
            assert_eq!("hyperlight_host::sandbox::uninitialized", logcall.target);

            // Log record 2

            let logcall = TEST_LOGGER.get_log_call(1).unwrap();
            assert_eq!(Level::Trace, logcall.level);
            assert_eq!(logcall.args, "-> UninitializedSandbox::new;");
            assert_eq!("tracing::span::active", logcall.target);

            // Log record 3

            let logcall = TEST_LOGGER.get_log_call(2).unwrap();
            assert_eq!(Level::Error, logcall.level);
            #[cfg(target_os = "windows")]
            assert!(logcall.args.starts_with(
                "error=Error The system cannot find the file specified. (os error 2) File Path"
            ));
            #[cfg(not(target_os = "windows"))]
            assert!(logcall
                .args
                .starts_with("error=Error No such file or directory (os error 2) File Path"));
            assert_eq!("hyperlight_host::sandbox::uninitialized", logcall.target);

            // Log record 4

            let logcall = TEST_LOGGER.get_log_call(3).unwrap();
            assert_eq!(Level::Trace, logcall.level);
            assert_eq!(logcall.args, "<- UninitializedSandbox::new;");
            assert_eq!("tracing::span::active", logcall.target);

            // Log record 6

            let logcall = TEST_LOGGER.get_log_call(4).unwrap();
            assert_eq!(Level::Trace, logcall.level);
            assert_eq!(logcall.args, "-- UninitializedSandbox::new;");
            assert_eq!("tracing::span", logcall.target);
        }
        {
            // test to ensure an invalid binary logs & traces properly
            TEST_LOGGER.clear_log_calls();
            TEST_LOGGER.set_max_level(log::LevelFilter::Info);

            let mut valid_binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            valid_binary_path.push("src");
            valid_binary_path.push("sandbox");
            valid_binary_path.push("initialized.rs");

            let sbox = UninitializedSandbox::new(
                valid_binary_path.into_os_string().into_string().unwrap(),
                None,
                None,
            );
            assert!(sbox.is_err());

            // There should be five calls again as we changed the log LevelFilter
            // to Info. We should see the 1 info level log seen in records 1 above.

            // We should then see the span and the info log record from pe_info
            // and then finally the 2 errors from pe info and sandbox as the
            // error result is propagated back up the call stack

            let num_calls = TEST_LOGGER.num_log_calls();
            assert_eq!(5, num_calls);

            // Log record 1

            let logcall = TEST_LOGGER.get_log_call(0).unwrap();
            assert_eq!(Level::Info, logcall.level);

            assert!(logcall
                .args
                .starts_with("UninitializedSandbox::new; bin_path"));
            assert_eq!("hyperlight_host::sandbox::uninitialized", logcall.target);

            // Log record 3

            let logcall = TEST_LOGGER.get_log_call(1).unwrap();
            assert_eq!(Level::Info, logcall.level);
            assert!(logcall.args.starts_with("from_file; filename="));
            assert_eq!("hyperlight_host::mem::pe::pe_info", logcall.target);

            // Log record 4

            let logcall = TEST_LOGGER.get_log_call(2).unwrap();
            assert_eq!(Level::Info, logcall.level);
            assert!(logcall.args.starts_with("Loading PE file from"));
            assert_eq!("hyperlight_host::mem::pe::pe_info", logcall.target);

            // Log record 5

            let logcall = TEST_LOGGER.get_log_call(3).unwrap();
            assert_eq!(Level::Error, logcall.level);
            assert!(logcall
                .args
                .starts_with("error=Malformed entity: DOS header is malformed"));
            assert_eq!("hyperlight_host::mem::pe::pe_info", logcall.target);

            // Log record 6

            let logcall = TEST_LOGGER.get_log_call(4).unwrap();
            assert_eq!(Level::Error, logcall.level);
            assert!(logcall
                .args
                .starts_with("error=Malformed entity: DOS header is malformed"));
            assert_eq!("hyperlight_host::sandbox::uninitialized", logcall.target);
        }
        {
            TEST_LOGGER.clear_log_calls();
            TEST_LOGGER.set_max_level(log::LevelFilter::Error);

            // Now we have set the max level to error, so we should not see any log calls as the following should not create an error

            let sbox = UninitializedSandbox::new(simple_guest_path().unwrap(), None, None);

            let sbox = sbox.unwrap();
            let _ = sbox.initialize::<fn(&mut UninitializedSandbox<'_>) -> Result<()>>(None);

            let num_calls = TEST_LOGGER.num_log_calls();
            assert_eq!(0, num_calls);
        }
    }
}
