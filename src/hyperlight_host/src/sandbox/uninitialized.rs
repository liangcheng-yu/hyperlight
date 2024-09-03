use std::ffi::c_void;
use std::fmt::Debug;
use std::ops::Add;
use std::option::Option;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tracing::{instrument, Span};

use super::host_funcs::{default_writer_func, HostFuncsWrapper};
use super::mem_mgr::MemMgrWrapper;
use super::run_options::SandboxRunOptions;
use super::uninitialized_evolve::{evolve_impl_multi_use, evolve_impl_single_use, ExecutionMode};
use crate::error::HyperlightError::{CallEntryPointIsInProcOnly, GuestBinaryShouldBeAFile};
use crate::func::host_functions::HostFunction1;
use crate::mem::mgr::{SandboxMemoryManager, STACK_COOKIE_LEN};
use crate::mem::pe::pe_info::PEInfo;
use crate::mem::ptr::RawPtr;
use crate::sandbox::{SandboxConfiguration, WrapperGetter};
use crate::sandbox_state::sandbox::EvolvableSandbox;
use crate::sandbox_state::transition::{MutatingCallback, Noop};
use crate::{debug, log_then_return, new_error, MultiUseSandbox, Result, SingleUseSandbox};

/// A preliminary `Sandbox`, not yet ready to execute guest code.
///
/// Prior to initializing a full-fledged `Sandbox`, you must create one of
/// these `UninitializedSandbox`es with the `new` function, register all the
/// host-implemented functions you need to be available to the guest, then
/// call  `evolve` to transform your
/// `UninitializedSandbox` into an initialized `Sandbox`.
#[derive(Clone)]
pub struct UninitializedSandbox {
    /// Registered host functions
    pub(crate) host_funcs: Arc<Mutex<HostFuncsWrapper>>,
    /// The memory manager for the sandbox.
    pub(crate) mgr: MemMgrWrapper,
    pub(crate) run_from_process_memory: bool,
    pub(crate) max_initialization_time: Duration,
    pub(crate) max_execution_time: Duration,
    pub(crate) max_wait_for_cancellation: Duration,
    /// Whether we're running in the context of C# code.
    ///
    /// This is a hack.
    pub(crate) is_csharp: bool,
}

impl WrapperGetter<'_> for UninitializedSandbox {
    fn get_mgr_wrapper(&self) -> &MemMgrWrapper {
        &self.mgr
    }
    fn get_mgr_wrapper_mut(&mut self) -> &mut MemMgrWrapper {
        &mut self.mgr
    }
}

impl<'a> crate::sandbox_state::sandbox::UninitializedSandbox<'a> for UninitializedSandbox {
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_uninitialized_sandbox(&self) -> &crate::sandbox::UninitializedSandbox {
        self
    }

    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_uninitialized_sandbox_mut(&mut self) -> &mut crate::sandbox::UninitializedSandbox {
        self
    }
}

impl Debug for UninitializedSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UninitializedSandbox")
            .field("memory_layout", &self.mgr.unwrap_mgr().layout)
            .finish()
    }
}

impl crate::sandbox_state::sandbox::Sandbox for UninitializedSandbox {
    fn check_stack_guard(&self) -> Result<bool> {
        self.mgr.check_stack_guard()
    }
}

impl<'a, F>
    EvolvableSandbox<
        UninitializedSandbox,
        SingleUseSandbox<'a>,
        MutatingCallback<'a, UninitializedSandbox, F>,
    > for UninitializedSandbox
where
    F: FnOnce(&mut UninitializedSandbox) -> Result<()> + 'a,
{
    /// Evolve `self` into a `SingleUseSandbox`, executing a caller-provided
    /// callback during the transition process.
    ///
    /// If you need to do this transition without a callback, use the
    /// `EvolvableSandbox` implementation that takes a `Noop`.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn evolve(
        self,
        tsn: MutatingCallback<'a, UninitializedSandbox, F>,
    ) -> Result<SingleUseSandbox<'a>> {
        let cb_box = {
            let cb = move |u_sbox: &mut UninitializedSandbox| tsn.call(u_sbox);
            Box::new(cb)
        };
        let i_sbox = evolve_impl_single_use(self, Some(cb_box))?;
        Ok(i_sbox)
    }
}

impl<'a, F>
    EvolvableSandbox<
        UninitializedSandbox,
        MultiUseSandbox<'a>,
        MutatingCallback<'a, UninitializedSandbox, F>,
    > for UninitializedSandbox
where
    F: FnOnce(&mut UninitializedSandbox) -> Result<()> + 'a,
{
    /// Evolve `self` into a `Sandbox`, executing a caller-provided
    /// callback during the transition process.
    ///
    /// If you need to do this transition without a callback, use the
    /// `EvolvableSandbox` implementation that takes a `Noop`.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn evolve(
        self,
        tsn: MutatingCallback<'a, UninitializedSandbox, F>,
    ) -> Result<MultiUseSandbox<'a>> {
        let cb_box = {
            let cb = move |u_sbox: &mut UninitializedSandbox| tsn.call(u_sbox);
            Box::new(cb)
        };

        let i_sbox = evolve_impl_multi_use(self, Some(cb_box))?;
        Ok(i_sbox)
    }
}

impl<'a>
    EvolvableSandbox<
        UninitializedSandbox,
        SingleUseSandbox<'a>,
        Noop<UninitializedSandbox, SingleUseSandbox<'a>>,
    > for UninitializedSandbox
{
    /// Evolve `self` to a `SingleUseSandbox` without any additional metadata.
    ///
    /// If you want to pass a callback to this state transition so you can
    /// run your own code during the transition, use the `EvolvableSandbox`
    /// implementation that accepts a `MutatingCallback`
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn evolve(
        self,
        _: Noop<UninitializedSandbox, SingleUseSandbox<'a>>,
    ) -> Result<SingleUseSandbox<'a>> {
        // TODO: the following if statement is to stop evolve_impl being called
        // when we run in proc (it ends up calling the entrypoint in the guest
        // twice)
        // Since we are not using the NOOP version of evolve in Hyperlight WASM
        // we can use the if statement below to avoid the call to evolve_impl
        // Once we fix up the Hypervisor C API this should be removed and
        // replaced with the code commented out on line 106
        let i_sbox = if self.is_csharp {
            Ok(SingleUseSandbox::from_uninit(self, ExecutionMode::CSharp))
        } else {
            evolve_impl_single_use(self, None)
        }?;
        Ok(i_sbox)
    }
}

impl<'a>
    EvolvableSandbox<
        UninitializedSandbox,
        MultiUseSandbox<'a>,
        Noop<UninitializedSandbox, MultiUseSandbox<'a>>,
    > for UninitializedSandbox
{
    /// Evolve `self` to a `MultiUseSandbox` without any additional metadata.
    ///
    /// If you want to pass a callback to this state transition so you can
    /// run your own code during the transition, use the `EvolvableSandbox`
    /// implementation that accepts a `MutatingCallback`
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn evolve(
        self,
        _: Noop<UninitializedSandbox, MultiUseSandbox<'a>>,
    ) -> Result<MultiUseSandbox<'a>> {
        // TODO: the following if statement is to stop evovle_impl being called
        // when we run in proc (it ends up calling the entrypoint in the guest
        // twice)
        //
        // Since we are not using the NOOP version of evolve in Hyperlight WASM
        // we can use the if statement below to avoid the call to evolve_impl
        // Once we fix up the Hypervisor C API this should be removed and
        // replaced with the code commented out on line 106
        let i_sbox = if self.is_csharp {
            Ok(MultiUseSandbox::from_uninit(self, ExecutionMode::CSharp))
        } else {
            evolve_impl_multi_use(self, None)
        }?;
        Ok(i_sbox)
    }
}

/// A `GuestBinary` is either a buffer containing the binary or a path to the binary
#[derive(Debug)]
pub enum GuestBinary {
    /// A buffer containing the guest binary
    Buffer(Vec<u8>),
    /// A path to the guest binary
    FilePath(String),
}

impl<'a> UninitializedSandbox {
    /// Create a new sandbox configured to run the binary at path
    /// `bin_path`.
    ///
    /// The instrument attribute is used to generate tracing spans and also to emit an error should the Result be an error.
    /// The skip attribute is used to skip the guest binary from being printed in the tracing span.
    /// The name attribute is used to name the tracing span.
    /// The err attribute is used to emit an error should the Result be an error, it uses the std::`fmt::Debug trait` to print the error.
    #[instrument(
        err(Debug),
        skip(guest_binary, host_print_writer),
        parent = Span::current()
    )]
    pub fn new(
        guest_binary: GuestBinary,
        cfg: Option<SandboxConfiguration>,
        sandbox_run_options: Option<SandboxRunOptions>,
        host_print_writer: Option<&dyn HostFunction1<'a, String, i32>>,
    ) -> Result<Self> {
        // If the guest binary is a file make sure it exists

        let guest_binary = match guest_binary {
            GuestBinary::FilePath(binary_path) => {
                let path = Path::new(&binary_path).canonicalize()?;
                path.try_exists()?;
                GuestBinary::FilePath(path.to_str().unwrap().to_string())
            }
            GuestBinary::Buffer(buffer) => GuestBinary::Buffer(buffer),
        };

        let run_opts = sandbox_run_options.unwrap_or_default();

        let run_from_process_memory = run_opts.is_in_memory();
        let run_from_guest_binary = run_opts.is_run_from_guest_binary();

        let sandbox_cfg = cfg.unwrap_or_default();
        let mut mem_mgr_wrapper = {
            let mut mgr = UninitializedSandbox::load_guest_binary(
                sandbox_cfg,
                &guest_binary,
                run_from_process_memory,
                run_from_guest_binary,
            )?;
            let stack_guard = Self::create_stack_guard();
            mgr.set_stack_guard(&stack_guard)?;
            MemMgrWrapper::new(mgr, stack_guard)
        };

        mem_mgr_wrapper.write_memory_layout(run_from_process_memory)?;

        let host_funcs = Arc::new(Mutex::new(HostFuncsWrapper::default()));

        let mut sandbox = Self {
            host_funcs,
            mgr: mem_mgr_wrapper,
            run_from_process_memory,
            is_csharp: false,
            max_initialization_time: Duration::from_millis(
                sandbox_cfg.get_max_initialization_time() as u64,
            ),
            max_execution_time: Duration::from_millis(sandbox_cfg.get_max_execution_time() as u64),
            max_wait_for_cancellation: Duration::from_millis(
                sandbox_cfg.get_max_wait_for_cancellation() as u64,
            ),
        };

        // If we were passed a writer for host print register it otherwise use the default.
        match host_print_writer {
            Some(writer_func) => {
                #[allow(clippy::arc_with_non_send_sync)]
                let writer_func = Arc::new(Mutex::new(writer_func));
                writer_func
                    .lock()
                    .map_err(|e| new_error!("Error Locking {:?}", e))?
                    .register(&mut sandbox, "HostPrint")?;
            }
            None => {
                let default_writer = Arc::new(Mutex::new(default_writer_func));
                default_writer.register(&mut sandbox, "HostPrint")?;
            }
        }

        debug!("Sandbox created:  {:#?}", sandbox);

        Ok(sandbox)
    }

    /// Get a reference to the internally-stored `SandboxMemoryManager`.
    ///
    /// TODO: remove this after the C API function `sandbox_get_memory_mgr`
    /// is removed.
    pub fn get_mem_mgr_ref(&self) -> &SandboxMemoryManager {
        self.get_mgr_wrapper().as_ref()
    }

    /// Get a mutable reference to the internally-stored
    /// `SandboxMemoryManager`
    #[cfg(target_os = "windows")]
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    pub(crate) fn get_mem_mgr_mut(&mut self) -> &mut SandboxMemoryManager {
        self.get_mgr_wrapper_mut().as_mut()
    }

    /// Set the internal flag to indicate this `UninitializedSandbox`
    /// is running in the context of C# code.
    ///
    /// This flag is used to indicate that Rust code should not call the
    /// guest's initialise function, since it expects C# code to do so
    /// manually.
    //TODO: Remove this once the port to the C API is complete
    pub fn set_is_csharp(&mut self) {
        self.is_csharp = true
    }

    /// Clone the internally-stored `Arc` holding the `HostFuncsWrapper`
    /// managed by `self`, then return it.
    // TODO: This function should not be public it is only used publically in the tests for the C API
    pub fn get_host_funcs(&self) -> Arc<Mutex<HostFuncsWrapper>> {
        self.host_funcs.clone()
    }
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn create_stack_guard() -> [u8; STACK_COOKIE_LEN] {
        rand::random::<[u8; STACK_COOKIE_LEN]>()
    }

    /// Call the entry point inside this `Sandbox` and return `Ok(())` if
    /// the entry point returned successfully. This function only applies to
    /// sandboxes with in-process mode turned on (e.g.
    /// `SandboxRunOptions::RunInProcess` passed as run options to the
    /// `UninitializedSandbox::new` function). If in-process mode is not
    /// turned on this function does nothing and immediately returns an `Err`.
    ///
    /// # Safety
    ///
    /// The given `peb_address` parameter must be an address in the guest
    /// memory corresponding to the start of the process
    /// environment block (PEB). If running with in-process mode, it must
    /// be an address into the host memory that points to the PEB.
    ///
    /// Additionally, `page_size` must correspond to the operating system's
    /// chosen size of a virtual memory page.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    //TODO:(#1029) Once CAPI is complete this should be pub(super)
    pub unsafe fn call_entry_point(
        &self,
        peb_address: RawPtr,
        seed: u64,
        page_size: u32,
    ) -> Result<()> {
        if !self.run_from_process_memory {
            log_then_return!(CallEntryPointIsInProcOnly());
        }
        type EntryPoint = extern "C" fn(i64, u64, u32, u32) -> i32;
        let entry_point: EntryPoint = {
            let addr = {
                let mgr = self.get_mgr_wrapper().as_ref();
                let offset = mgr.entrypoint_offset;
                mgr.load_addr.clone().add(offset)
            };

            let fn_location = u64::from(addr) as *const c_void;
            unsafe { std::mem::transmute(fn_location) }
        };
        let peb_i64 = i64::try_from(u64::from(peb_address))?;
        let max_log_level = log::max_level() as u32;
        entry_point(peb_i64, seed, page_size, max_log_level);
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
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub(super) fn load_guest_binary(
        cfg: SandboxConfiguration,
        guest_binary: &GuestBinary,
        run_from_process_memory: bool,
        run_from_guest_binary: bool,
    ) -> Result<SandboxMemoryManager> {
        let mut pe_info = match guest_binary {
            GuestBinary::FilePath(bin_path_str) => PEInfo::from_file(bin_path_str)?,
            GuestBinary::Buffer(buffer) => PEInfo::new(buffer)?,
        };

        if run_from_guest_binary {
            let path = match guest_binary {
                GuestBinary::FilePath(bin_path_str) => bin_path_str,
                GuestBinary::Buffer(_) => {
                    log_then_return!(GuestBinaryShouldBeAFile());
                }
            };
            // TODO: This produces the wrong error message on Linux and is possibly obsfucating the real error on Windows
            SandboxMemoryManager::load_guest_binary_using_load_library(
                cfg,
                path,
                &mut pe_info,
                run_from_process_memory,
            )
            .map_err(|e: crate::HyperlightError| {
                new_error!(
                    "Only one instance of Sandbox is allowed when running from guest binary: {:?}",
                    e
                )
            })
        } else {
            SandboxMemoryManager::load_guest_binary_into_memory(
                cfg,
                &mut pe_info,
                run_from_process_memory,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::{fs, thread};

    use crossbeam_queue::ArrayQueue;
    use hyperlight_common::flatbuffer_wrappers::function_types::{ParameterValue, ReturnValue};
    use hyperlight_testing::logger::{Logger as TestLogger, LOGGER as TEST_LOGGER};
    use hyperlight_testing::simple_guest_as_string;
    use hyperlight_testing::tracing_subscriber::TracingSubscriber as TestSubcriber;
    use log::Level;
    use serde_json::{Map, Value};
    use serial_test::serial;
    use tempfile::NamedTempFile;
    use tracing::Level as tracing_level;
    use tracing_core::callsite::rebuild_interest_cache;
    use tracing_core::Subscriber;
    use uuid::Uuid;

    use crate::func::host_functions::{HostFunction1, HostFunction2};
    use crate::sandbox::uninitialized::GuestBinary;
    use crate::sandbox::{SandboxConfiguration, WrapperGetter};
    use crate::sandbox_state::sandbox::EvolvableSandbox;
    use crate::sandbox_state::transition::{MutatingCallback, Noop};
    use crate::testing::log_values::{test_value_as_str, try_to_strings};
    #[cfg(target_os = "windows")]
    use crate::SandboxRunOptions;
    use crate::{MultiUseSandbox, Result, UninitializedSandbox};

    #[test]
    fn test_new_sandbox() {
        // Guest Binary exists at path

        let binary_path = simple_guest_as_string().unwrap();
        let sandbox =
            UninitializedSandbox::new(GuestBinary::FilePath(binary_path.clone()), None, None, None);
        assert!(sandbox.is_ok());

        // Guest Binary does not exist at path

        let binary_path_does_not_exist = binary_path.trim_end_matches(".exe").to_string();
        let uninitialized_sandbox = UninitializedSandbox::new(
            GuestBinary::FilePath(binary_path_does_not_exist),
            None,
            None,
            None,
        );
        assert!(uninitialized_sandbox.is_err());

        // Non default memory configuration
        let cfg = {
            let mut cfg = SandboxConfiguration::default();
            cfg.set_input_data_size(0x1000);
            cfg.set_output_data_size(0x1000);
            cfg.set_host_function_definition_size(0x1000);
            cfg.set_host_exception_size(0x1000);
            cfg.set_guest_error_buffer_size(0x1000);
            cfg.set_stack_size(0x1000);
            cfg.set_heap_size(0x1000);
            cfg.set_max_execution_time(Duration::from_millis(1001));
            cfg.set_max_execution_cancel_wait_time(Duration::from_millis(9));
            Some(cfg)
        };

        let uninitialized_sandbox =
            UninitializedSandbox::new(GuestBinary::FilePath(binary_path.clone()), cfg, None, None);
        assert!(uninitialized_sandbox.is_ok());

        let uninitialized_sandbox =
            UninitializedSandbox::new(GuestBinary::FilePath(binary_path), None, None, None)
                .unwrap();

        // Get a Sandbox from an uninitialized sandbox without a call back function

        let _sandbox: MultiUseSandbox<'_> = uninitialized_sandbox.evolve(Noop::default()).unwrap();

        // Test with  init callback function
        // TODO: replace this with a test that registers and calls functions once we have that functionality

        let received_msg = Arc::new(Mutex::new(String::new()));
        let received_msg_clone = received_msg.clone();

        let writer = move |msg| {
            let mut received_msg = received_msg_clone.lock().unwrap();
            *received_msg = msg;
            Ok(0)
        };

        let writer_func = Arc::new(Mutex::new(writer));

        let mut uninitialized_sandbox = UninitializedSandbox::new(
            GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing")),
            None,
            None,
            None,
        )
        .expect("Failed to create sandbox");

        writer_func
            .register(&mut uninitialized_sandbox, "HostPrint")
            .expect("Failed to register writer function");

        fn init(uninitialized_sandbox: &mut UninitializedSandbox) -> Result<()> {
            uninitialized_sandbox
                .host_funcs
                .lock()?
                .host_print("test".to_string())?;

            Ok(())
        }

        let sandbox: Result<MultiUseSandbox<'_>> =
            uninitialized_sandbox.evolve(MutatingCallback::from(init));
        assert!(sandbox.is_ok());

        drop(sandbox);

        assert_eq!(received_msg.lock().unwrap().as_str(), "test");

        // Test with a valid guest binary buffer

        let binary_path = simple_guest_as_string().unwrap();
        let sandbox = UninitializedSandbox::new(
            GuestBinary::Buffer(fs::read(binary_path).unwrap()),
            None,
            None,
            None,
        );
        assert!(sandbox.is_ok());

        // Test with a invalid guest binary buffer

        let binary_path = simple_guest_as_string().unwrap();
        let mut bytes = fs::read(binary_path).unwrap();
        let _ = bytes.split_off(100);
        let sandbox = UninitializedSandbox::new(GuestBinary::Buffer(bytes), None, None, None);
        assert!(sandbox.is_err());

        // Test with a valid guest binary buffer when trying to load library
        #[cfg(target_os = "windows")]
        {
            let binary_path = simple_guest_as_string().unwrap();
            let sandbox = UninitializedSandbox::new(
                GuestBinary::Buffer(fs::read(binary_path).unwrap()),
                None,
                Some(SandboxRunOptions::RunInProcess(true)),
                None,
            );
            assert!(sandbox.is_err());
        }
    }

    #[test]
    fn test_load_guest_binary_manual() {
        let cfg = SandboxConfiguration::default();

        let simple_guest_path = simple_guest_as_string().unwrap();

        UninitializedSandbox::load_guest_binary(
            cfg,
            &GuestBinary::FilePath(simple_guest_path),
            false,
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_stack_guard() {
        let simple_guest_path = simple_guest_as_string().unwrap();
        let sbox =
            UninitializedSandbox::new(GuestBinary::FilePath(simple_guest_path), None, None, None)
                .unwrap();
        let res = sbox.get_mgr_wrapper().check_stack_guard();
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
                GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing")),
                None,
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

            let sandbox: Result<MultiUseSandbox<'_>> = usbox.evolve(MutatingCallback::from(init));
            assert!(sandbox.is_ok());
            let sandbox = sandbox.unwrap();

            let host_funcs = sandbox._host_funcs.lock();

            assert!(host_funcs.is_ok());

            let res = host_funcs
                .unwrap()
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

            let sandbox: Result<MultiUseSandbox<'_>> = usbox.evolve(MutatingCallback::from(init));
            assert!(sandbox.is_ok());
            let sandbox = sandbox.unwrap();

            let host_funcs = sandbox._host_funcs.lock();

            assert!(host_funcs.is_ok());

            let res = host_funcs
                .unwrap()
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

            let sandbox: Result<MultiUseSandbox<'_>> = usbox.evolve(MutatingCallback::from(init));
            assert!(sandbox.is_ok());
            let sandbox = sandbox.unwrap();

            let host_funcs = sandbox._host_funcs.lock();

            assert!(host_funcs.is_ok());

            let res = host_funcs.unwrap().call_host_function("test2", vec![]);
            assert!(res.is_err());
        }

        // calling a function that doesn't exist
        {
            let usbox = uninitialized_sandbox();
            let sandbox: Result<MultiUseSandbox<'_>> = usbox.evolve(MutatingCallback::from(init));
            assert!(sandbox.is_ok());
            let sandbox = sandbox.unwrap();

            let host_funcs = sandbox._host_funcs.lock();

            assert!(host_funcs.is_ok());

            let res = host_funcs.unwrap().call_host_function("test4", vec![]);
            assert!(res.is_err());
        }
    }

    #[test]
    #[serial]
    fn test_load_guest_binary_load_lib() {
        let cfg = SandboxConfiguration::default();
        let simple_guest_path = simple_guest_as_string().unwrap();
        let mgr_res = UninitializedSandbox::load_guest_binary(
            cfg,
            &GuestBinary::FilePath(simple_guest_path),
            true,
            true,
        );
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

        let received_msg = Arc::new(Mutex::new(String::new()));
        let received_msg_clone = received_msg.clone();

        let writer = move |msg| {
            let mut received_msg = received_msg_clone.lock().unwrap();
            *received_msg = msg;
            Ok(0)
        };

        let hostfunc = Arc::new(Mutex::new(writer));

        let sandbox = UninitializedSandbox::new(
            GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing")),
            None,
            None,
            Some(&hostfunc),
        )
        .expect("Failed to create sandbox");

        let host_funcs = sandbox.host_funcs.lock();

        assert!(host_funcs.is_ok());

        host_funcs.unwrap().host_print("test".to_string()).unwrap();

        drop(sandbox);

        assert_eq!(received_msg.lock().unwrap().as_str(), "test");

        // There may be cases where a mutable reference to the captured variable is not required to be used outside the closue
        // e.g. if the function is writing to a file or a socket etc.

        // writer as a FnMut closure mutating a captured variable but not trying to access the captured variable

        // This seems more realistic as the client is creating a file to be written to in the closure
        // and then accessing the file a different handle.
        // The problem is that captured_file still needs static lifetime so even though we can access the data through the second file handle
        // this still does not work as the captured_file is dropped at the end of the function

        let captured_file = Arc::new(Mutex::new(NamedTempFile::new().unwrap()));
        let capture_file_clone = captured_file.clone();

        let capture_file_lock = captured_file.lock().unwrap();
        let mut file = capture_file_lock.reopen().unwrap();
        drop(capture_file_lock);

        let writer = move |msg: String| -> Result<i32> {
            let mut captured_file = capture_file_clone.lock().unwrap();
            captured_file.write_all(msg.as_bytes()).unwrap();
            Ok(0)
        };

        let writer_func = Arc::new(Mutex::new(writer));

        let sandbox = UninitializedSandbox::new(
            GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing")),
            None,
            None,
            Some(&writer_func),
        )
        .expect("Failed to create sandbox");

        let host_funcs = sandbox.host_funcs.lock();

        assert!(host_funcs.is_ok());

        host_funcs.unwrap().host_print("test2".to_string()).unwrap();

        let mut buffer = String::new();
        file.read_to_string(&mut buffer).unwrap();
        assert_eq!(buffer, "test2");

        // writer as a function

        fn fn_writer(msg: String) -> Result<i32> {
            assert_eq!(msg, "test2");
            Ok(0)
        }

        let writer_func = Arc::new(Mutex::new(fn_writer));
        let sandbox = UninitializedSandbox::new(
            GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing")),
            None,
            None,
            Some(&writer_func),
        )
        .expect("Failed to create sandbox");

        let host_funcs = sandbox.host_funcs.lock();

        assert!(host_funcs.is_ok());

        host_funcs.unwrap().host_print("test2".to_string()).unwrap();

        // writer as a method

        let mut test_host_print = TestHostPrint::new();

        // create a closure over the struct method

        let writer_closure = move |s| test_host_print.write(s);

        let writer_method = Arc::new(Mutex::new(writer_closure));

        let sandbox = UninitializedSandbox::new(
            GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing")),
            None,
            None,
            Some(&writer_method),
        )
        .expect("Failed to create sandbox");

        let host_funcs = sandbox.host_funcs.lock();

        assert!(host_funcs.is_ok());

        host_funcs.unwrap().host_print("test3".to_string()).unwrap();
    }

    struct TestHostPrint {}

    impl TestHostPrint {
        fn new() -> Self {
            TestHostPrint {}
        }

        fn write(&mut self, msg: String) -> Result<i32> {
            assert_eq!(msg, "test3");
            Ok(0)
        }
    }

    #[test]
    fn check_create_and_use_sandbox_on_different_threads() {
        let unintializedsandbox_queue = Arc::new(ArrayQueue::<UninitializedSandbox>::new(10));
        let sandbox_queue = Arc::new(ArrayQueue::<MultiUseSandbox<'_>>::new(10));

        for i in 0..10 {
            let simple_guest_path = simple_guest_as_string().expect("Guest Binary Missing");
            let unintializedsandbox = {
                let err_string = format!("failed to create UninitializedSandbox {i}");
                let err_str = err_string.as_str();
                UninitializedSandbox::new(
                    GuestBinary::FilePath(simple_guest_path),
                    None,
                    None,
                    None,
                )
                .expect(err_str)
            };

            {
                let err_string = format!("Failed to push UninitializedSandbox {i}");
                let err_str = err_string.as_str();

                unintializedsandbox_queue
                    .push(unintializedsandbox)
                    .expect(err_str);
            }
        }

        let thread_handles = (0..10)
            .map(|i| {
                let uq = unintializedsandbox_queue.clone();
                let sq = sandbox_queue.clone();
                thread::spawn(move || {
                    let uninitialized_sandbox = uq.pop().unwrap_or_else(|| {
                        panic!("Failed to pop UninitializedSandbox thread {}", i)
                    });

                    let host_funcs = uninitialized_sandbox.host_funcs.lock();

                    assert!(host_funcs.is_ok());

                    host_funcs
                        .unwrap()
                        .host_print(format!("Print from UninitializedSandbox on Thread {}\n", i))
                        .unwrap();

                    let sandbox = uninitialized_sandbox
                        .evolve(Noop::default())
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
                    let sandbox = sq
                        .pop()
                        .unwrap_or_else(|| panic!("Failed to pop Sandbox thread {}", i));

                    let host_funcs = sandbox._host_funcs.lock();

                    assert!(host_funcs.is_ok());

                    host_funcs
                        .unwrap()
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
    // Tests that trace data are emitted when a trace subscriber is set
    // this test is ignored because it is incompatible with other tests , specifically those which require a logger for tracing
    // marking  this test as ignored means that running `cargo test` will not run this test but will allow a developer who runs that command
    // from their workstation to be successful without needed to know about test interdependencies
    // this test will be run explcitly as a part of the CI pipeline
    #[ignore]
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

            let mut binary_path = simple_guest_as_string().unwrap();
            binary_path.push_str("does_not_exist");

            let sbox =
                UninitializedSandbox::new(GuestBinary::FilePath(binary_path), None, None, None);
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
            assert_eq!(span_metadata.name(), "new");

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
                    "IOError(Os { code: 2, kind: NotFound, message: \"The system cannot find the file specified.\" }";
                #[cfg(not(target_os = "windows"))]
                    let expected_error = "IOError(Os { code: 2, kind: NotFound, message: \"No such file or directory\" }";

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
    #[ignore]
    // Tests that traces are emitted as log records when there is no trace
    // subscriber configured.
    fn test_log_trace() {
        {
            TestLogger::initialize_test_logger();
            TEST_LOGGER.set_max_level(log::LevelFilter::Trace);

            // This makes sure that the metadata interest cache is rebuilt so that
            // the log records are emitted for the trace records

            rebuild_interest_cache();

            let mut invalid_binary_path = simple_guest_as_string().unwrap();
            invalid_binary_path.push_str("does_not_exist");

            let sbox = UninitializedSandbox::new(
                GuestBinary::FilePath(invalid_binary_path),
                None,
                None,
                None,
            );
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

            assert!(logcall.args.starts_with("new; cfg"));
            assert_eq!("hyperlight_host::sandbox::uninitialized", logcall.target);

            // Log record 2

            let logcall = TEST_LOGGER.get_log_call(1).unwrap();
            assert_eq!(Level::Trace, logcall.level);
            assert_eq!(logcall.args, "-> new;");
            assert_eq!("tracing::span::active", logcall.target);

            // Log record 3

            let logcall = TEST_LOGGER.get_log_call(2).unwrap();
            assert_eq!(Level::Error, logcall.level);
            assert!(logcall.args.starts_with("error=IOError(Os { code"));
            assert_eq!("hyperlight_host::sandbox::uninitialized", logcall.target);

            // Log record 4

            let logcall = TEST_LOGGER.get_log_call(3).unwrap();
            assert_eq!(Level::Trace, logcall.level);
            assert_eq!(logcall.args, "<- new;");
            assert_eq!("tracing::span::active", logcall.target);

            // Log record 6

            let logcall = TEST_LOGGER.get_log_call(4).unwrap();
            assert_eq!(Level::Trace, logcall.level);
            assert_eq!(logcall.args, "-- new;");
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
                GuestBinary::FilePath(valid_binary_path.into_os_string().into_string().unwrap()),
                None,
                None,
                None,
            );
            assert!(sbox.is_err());

            // There should be 2 calls this time when we change to the log
            // LevelFilter to Info.
            let num_calls = TEST_LOGGER.num_log_calls();
            assert_eq!(2, num_calls);

            // Log record 1

            let logcall = TEST_LOGGER.get_log_call(0).unwrap();
            assert_eq!(Level::Info, logcall.level);

            assert!(logcall.args.starts_with("new; cfg"));
            assert_eq!("hyperlight_host::sandbox::uninitialized", logcall.target);

            // Log record 2

            let logcall = TEST_LOGGER.get_log_call(1).unwrap();
            assert_eq!(Level::Error, logcall.level);
            assert!(logcall.args.starts_with("error=IOError"));
            assert_eq!("hyperlight_host::sandbox::uninitialized", logcall.target);
        }
        {
            TEST_LOGGER.clear_log_calls();
            TEST_LOGGER.set_max_level(log::LevelFilter::Error);

            let sbox = {
                let res = UninitializedSandbox::new(
                    GuestBinary::FilePath(simple_guest_as_string().unwrap()),
                    None,
                    None,
                    None,
                );
                res.unwrap()
            };
            let _: Result<MultiUseSandbox<'_>> = sbox.evolve(Noop::default());

            let num_calls = TEST_LOGGER.num_log_calls();

            assert_eq!(0, num_calls);
        }
    }
}
