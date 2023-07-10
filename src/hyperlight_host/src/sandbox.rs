use super::sandbox_run_options::SandboxRunOptions;
use crate::flatbuffers::hyperlight::generated::ErrorCode;
use crate::func::host::vals::{Parameters, Return, SupportedParameterOrReturnValue};
use crate::func::host::{Function1, HyperlightFunction};
use crate::guest::guest_log_data::GuestLogData;
use crate::guest::host_function_definition::HostFunctionDefinition;
use crate::guest::log_level::LogLevel;
use crate::hypervisor::Hypervisor;
use crate::mem::mgr::STACK_COOKIE_LEN;
use crate::mem::ptr::RawPtr;
use crate::mem::{
    config::SandboxMemoryConfiguration, mgr::SandboxMemoryManager, pe::pe_info::PEInfo,
};
#[cfg(target_os = "linux")]
use crate::{
    hypervisor::hyperv_linux::{self, HypervLinuxDriver},
    hypervisor::hypervisor_mem::HypervisorAddrs,
    hypervisor::kvm,
    hypervisor::kvm::KVMDriver,
    mem::layout::SandboxMemoryLayout,
    mem::ptr::GuestPtr,
    mem::ptr_offset::Offset,
};
use anyhow::{anyhow, bail, Result};
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use std::ffi::c_void;
use std::io::stdout;
use std::io::Write;
use std::ops::Add;
use std::option::Option;
use std::path::Path;
use std::sync::{Arc, Mutex};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

// In case its not obvious why there are separate is_supported_platform and is_hypervisor_present functions its because
// Hyperlight is designed to be able to run on a host that doesn't have a hypervisor.
// In that case, the sandbox will be in process, we plan on making this a dev only feature and fixing up Linux support
// so we should review the need for this function at that time.

/// Determine if this is a supported platform for Hyperlight
///
/// Returns a boolean indicating whether this is a supported platform.
///
pub(crate) fn is_supported_platform() -> bool {
    #[cfg(not(target_os = "linux"))]
    #[cfg(not(target_os = "windows"))]
    return false;

    true
}

enum OutBAction {
    Log,
    CallFunction,
    Abort,
}

impl From<u16> for OutBAction {
    fn from(val: u16) -> Self {
        match val {
            99 => OutBAction::Log,
            101 => OutBAction::CallFunction,
            102 => OutBAction::Abort,
            _ => OutBAction::Log,
        }
    }
}

/// Determine whether a suitable hypervisor is available to run
/// this sandbox.
///
//  Returns a boolean indicating whether a suitable hypervisor is present.

pub(crate) fn is_hypervisor_present() -> bool {
    #[cfg(target_os = "linux")]
    return hyperv_linux::is_hypervisor_present().unwrap_or(false)
        || kvm::is_hypervisor_present().is_ok();
    #[cfg(target_os = "windows")]
    //TODO: Implement this for Windows once Rust WHP support is merged.
    return true;
    #[cfg(not(target_os = "linux"))]
    #[cfg(not(target_os = "windows"))]
    false
}

/// Sandboxes are the primary mechanism to interact with VM partitions.
///
/// Prior to initializing a Sandbox, the caller must register all host functions
/// onto an UninitializedSandbox. Once all host functions have been registered,
/// the UninitializedSandbox can be initialized into a Sandbox through the
/// `initialize` method.
pub struct UnintializedSandbox<'a> {
    // Registered host functions
    host_functions: HashMap<String, HyperlightFunction<'a>>,
    // The memory manager for the sandbox.
    mem_mgr: SandboxMemoryManager,
    stack_guard: [u8; STACK_COOKIE_LEN],
}

/// The primary mechanism to interact with VM partitions that
/// run Hyperlight Sandboxes.
///
/// A Hyperlight Sandbox is a specialized VM environment
/// intended specifically for running Hyperlight guest processes.
#[allow(unused)]
pub struct Sandbox<'a> {
    // Registered host functions
    host_functions: HashMap<String, HyperlightFunction<'a>>,
    // The memory manager for the sandbox.
    mem_mgr: SandboxMemoryManager,
    stack_guard: [u8; STACK_COOKIE_LEN],
    uninit_sandbox: UnintializedSandbox<'a>,
}

impl<'a> std::fmt::Debug for Sandbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sandbox")
            .field("stack_guard", &self.stack_guard)
            .finish()
    }
}

impl<'a> std::fmt::Debug for UnintializedSandbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sandbox")
            .field("stack_guard", &self.stack_guard)
            .finish()
    }
}

impl<'a> UnintializedSandbox<'a> {
    /// Create a new sandbox configured to run the binary at path
    /// `bin_path`.
    pub fn new(
        bin_path: String,
        cfg: Option<SandboxMemoryConfiguration>,
        sandbox_run_options: Option<SandboxRunOptions>,
    ) -> Result<Self> {
        // Make sure the binary exists
        let path = Path::new(&bin_path).canonicalize()?;
        path.try_exists()?;

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
        let mut mem_mgr = UnintializedSandbox::load_guest_binary(
            mem_cfg,
            &bin_path,
            run_from_process_memory,
            run_from_guest_binary,
        )?;

        // <WriteMemoryLayout>
        let layout = mem_mgr.layout;
        let shared_mem = mem_mgr.get_shared_mem_mut();
        let mem_size = shared_mem.mem_size();
        layout.write(shared_mem, SandboxMemoryLayout::BASE_ADDRESS, mem_size)?;
        // </WriteMemoryLayout>

        let stack_guard = Self::create_stack_guard();
        mem_mgr.set_stack_guard(&stack_guard)?;

        // The default writer function is to write to stdout with green text.
        let default_writer_func = Arc::new(Mutex::new(|s: String| -> Result<()> {
            match atty::is(atty::Stream::Stdout) {
                false => {
                    stdout().write_all(s.as_bytes())?;
                    Ok(())
                }
                true => {
                    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
                    let mut color_spec = ColorSpec::new();
                    color_spec.set_fg(Some(Color::Green));
                    stdout.set_color(&color_spec)?;
                    stdout.write_all(s.as_bytes())?;
                    stdout.reset()?;
                    Ok(())
                }
            }
        }));

        let mut sandbox = Self {
            host_functions: HashMap::new(),
            mem_mgr,
            stack_guard,
        };

        default_writer_func.register(&mut sandbox, "writer_func")?;

        Ok(sandbox)
    }

    fn create_stack_guard() -> [u8; STACK_COOKIE_LEN] {
        rand::random::<[u8; STACK_COOKIE_LEN]>()
    }

    /// Check the stack guard against the stack guard cookie stored
    /// within `self`. Return `Ok(true)` if the guard cookie could
    /// be found and it matched `self.stack_guard`, `Ok(false)` if
    /// if could be found and did not match `self.stack_guard`, and
    /// `Err` if it could not be found or there was some other error.
    ///
    /// TODO: remove the dead code annotation after this is hooked up in
    /// https://github.com/deislabs/hyperlight/pull/727
    #[allow(dead_code)]
    fn check_stack_guard(&self) -> Result<bool> {
        self.mem_mgr.check_stack_guard(self.stack_guard)
    }

    /// Register a host function with the sandbox.
    pub fn register_host_function(
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

    /// Determine whether a suitable hypervisor is available to run
    /// this sandbox.
    ///
    /// Returns `Ok` with a boolean if it could be determined whether
    /// an appropriate hypervisor is available, and `Err` otherwise.
    pub fn is_hypervisor_present(&self) -> Result<bool> {
        // TODO: implement
        Ok(true)
    }

    /// TODO: This should be removed once we have a proper Sandbox with C API that provides all functionaliy
    /// It only exists to keep the C# code working for now
    ///
    pub(crate) fn get_mem_mgr(&self) -> SandboxMemoryManager {
        self.mem_mgr.clone()
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
    ///
    fn load_guest_binary(
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

    /// Check for a guest error and return an `Err` if one was found,
    /// and `Ok` if one was not found.
    /// TODO: remove this when we hook it up to the rest of the
    /// sandbox in https://github.com/deislabs/hyperlight/pull/727
    #[allow(dead_code)]
    fn check_for_guest_error(&self) -> Result<()> {
        let guest_err = self.mem_mgr.get_guest_error()?;
        match guest_err.code {
            ErrorCode::NoError => Ok(()),
            ErrorCode::OutbError => match self.mem_mgr.get_host_error()? {
                Some(host_err) => bail!("[OutB Error] {:?}: {:?}", guest_err.code, host_err),
                None => Ok(()),
            },
            ErrorCode::StackOverflow => {
                let err_msg = format!(
                    "[Stack Overflow] Guest Error: {:?}: {}",
                    guest_err.code, guest_err.message
                );
                error!("{}", err_msg);
                bail!(err_msg);
            }
            _ => {
                let err_msg = format!("Guest Error: {:?}: {}", guest_err.code, guest_err.message);
                error!("{}", err_msg);
                bail!(err_msg);
            }
        }
    }

    /// Initialize the `Sandbox` from an `UninitializedSandbox`.
    /// Receives a callback function to be called during initialization.
    #[allow(unused)]
    fn initialize<F: Fn(&mut UnintializedSandbox<'a>) -> Result<()>>(
        mut self,
        callback: Option<F>,
    ) -> Result<Sandbox<'a>> {
        if let Some(cb) = callback {
            cb(&mut self)?;
        }

        let mut sbox = Sandbox {
            host_functions: self.host_functions.clone(),
            mem_mgr: self.mem_mgr.clone(),
            stack_guard: self.stack_guard,
            uninit_sandbox: self,
        };

        Ok(sbox)
    }

    /// Host print function – an exception to normal calling functions
    /// as it can be called prior to initialization.
    pub(crate) fn host_print(&mut self, msg: String) -> Result<()> {
        let writer_func = self
            .host_functions
            .get_mut("writer_func")
            .ok_or_else(|| anyhow!("Host function 'writer_func' not found"))?;

        writer_func.lock().unwrap()(vec![SupportedParameterOrReturnValue::String(msg)].into())?;

        Ok(())
    }
}

impl<'a> Sandbox<'a> {
    /// Call a host print in the sandbox.
    #[allow(unused)]
    pub(crate) fn host_print(&mut self, msg: String) -> Result<()> {
        self.call_host_function(
            "writer_func",
            vec![SupportedParameterOrReturnValue::String(msg)].into(),
        )?;

        Ok(())
    }

    /// Call a host function in the sandbox.
    pub fn call_host_function(&mut self, name: &str, args: Parameters) -> Result<Return> {
        let func = self
            .host_functions
            .get(name)
            .ok_or_else(|| anyhow!("Host function {} not found", name))?;

        func.lock().unwrap()(args)
    }

    #[allow(unused)]
    pub(crate) fn handle_outb(&mut self, port: u16, byte: u8) -> Result<()> {
        match port.into() {
            OutBAction::Log => outb_log(&self.mem_mgr),
            OutBAction::CallFunction => {
                let call = self.mem_mgr.get_host_function_call()?;
                let name = call.function_name.clone();
                let args: Parameters = call.parameters.clone().try_into()?;
                let res = self.call_host_function(&name, args)?;
                self.mem_mgr
                    .write_response_from_host_method_call(&res.try_into()?)?;
                Ok(())
            }
            OutBAction::Abort => {
                // TODO
                todo!();
            }
            _ => {
                // TODO
                todo!();
            }
        }
    }
}

fn outb_log(mgr: &SandboxMemoryManager) -> Result<()> {
    let log_data: GuestLogData = mgr.read_guest_log_data()?;
    let log_level = &log_data.level;
    let message = format!(
        "{} [{}:{}] {}, {}\n",
        log_data.source, log_data.source_file, log_data.line, log_data.caller, log_data.message
    );
    match log_level {
        LogLevel::Trace => trace!("{}", message),
        LogLevel::Debug => debug!("{}", message),
        LogLevel::Information => info!("{}", message),
        LogLevel::Warning => warn!("{}", message),
        LogLevel::Error => error!("{}", message),
        LogLevel::Critical => error!("[CRITICAL] {}", message),
        // Do nothing if the log level is set to none
        LogLevel::None => (),
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::{is_hypervisor_present, outb_log, Sandbox, UnintializedSandbox};
    #[cfg(target_os = "windows")]
    use super::{outb_log, Sandbox, UnintializedSandbox};
    #[cfg(target_os = "linux")]
    use crate::hypervisor::hyperv_linux::test_cfg::TEST_CONFIG as HYPERV_TEST_CONFIG;
    #[cfg(target_os = "linux")]
    use crate::hypervisor::kvm::test_cfg::TEST_CONFIG as KVM_TEST_CONFIG;
    use crate::{
        func::host::{
            vals::{Parameters, SupportedParameterOrReturnValue},
            Function1, Function2,
        },
        guest::{guest_log_data::GuestLogData, log_level::LogLevel},
        mem::{config::SandboxMemoryConfiguration, mgr::SandboxMemoryManager},
        sandbox_run_options::SandboxRunOptions,
        testing::{logger::LOGGER, simple_guest_path, simple_guest_pe_info},
    };
    use anyhow::Result;
    use crossbeam_queue::ArrayQueue;
    use log::{set_logger, set_max_level, Level};
    use std::{
        io::{Read, Write},
        sync::{Arc, Mutex},
        thread,
    };
    use tempfile::NamedTempFile;
    #[test]
    // TODO: add support for testing on WHP
    #[cfg(target_os = "linux")]
    fn test_is_hypervisor_present() {
        // TODO: Handle requiring a stable API
        if HYPERV_TEST_CONFIG.hyperv_should_be_present || KVM_TEST_CONFIG.kvm_should_be_present {
            assert!(is_hypervisor_present());
        } else {
            assert!(!is_hypervisor_present());
        }
    }

    #[test]
    fn test_new_sandbox() {
        // Guest Binary exists at path

        let binary_path = simple_guest_path().unwrap();
        let sandbox = UnintializedSandbox::new(binary_path.clone(), None, None);
        assert!(sandbox.is_ok());

        // Guest Binary does not exist at path

        let binary_path_does_not_exist = binary_path.trim_end_matches(".exe").to_string();
        let uninitialized_sandbox =
            UnintializedSandbox::new(binary_path_does_not_exist, None, None);
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

        let uninitialized_sandbox = UnintializedSandbox::new(binary_path.clone(), Some(cfg), None);
        assert!(uninitialized_sandbox.is_ok());

        // Invalid sandbox_run_options

        let sandbox_run_options =
            SandboxRunOptions::RUN_FROM_GUEST_BINARY | SandboxRunOptions::RECYCLE_AFTER_RUN;

        let uninitialized_sandbox =
            UnintializedSandbox::new(binary_path.clone(), None, Some(sandbox_run_options));
        assert!(uninitialized_sandbox.is_err());

        let uninitialized_sandbox = UnintializedSandbox::new(binary_path, None, None);
        assert!(uninitialized_sandbox.is_ok());

        // Get a Sandbox from an uninitialized sandbox without a call back function

        let sandbox = uninitialized_sandbox
            .unwrap()
            .initialize::<fn(&mut UnintializedSandbox<'_>) -> Result<()>>(None);
        assert!(sandbox.is_ok());

        // Test with  init callback function
        // TODO: replace this with a test that registers and calls functions once we have that functionality

        let mut received_msg = String::new();

        let writer = |msg| {
            received_msg = msg;
            Ok(())
        };

        let writer_func = Arc::new(Mutex::new(writer));

        let mut uninitialized_sandbox = UnintializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            None,
        )
        .expect("Failed to create sandbox");

        writer_func
            .register(&mut uninitialized_sandbox, "writer_func")
            .expect("Failed to register writer function");

        fn init(uninitialized_sandbox: &mut UnintializedSandbox) -> Result<()> {
            uninitialized_sandbox.host_print("test".to_string())
        }

        let sandbox = uninitialized_sandbox.initialize(Some(init));
        assert!(sandbox.is_ok());

        drop(sandbox);

        assert_eq!(&received_msg, "test");
    }

    #[test]
    fn test_host_functions() {
        let uninitialized_sandbox = || {
            UnintializedSandbox::new(
                simple_guest_path().expect("Guest Binary Missing"),
                None,
                None,
            )
            .unwrap()
        };
        fn init(_: &mut UnintializedSandbox) -> Result<()> {
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
                .call_host_function(
                    "test0",
                    Parameters(vec![SupportedParameterOrReturnValue::Int(1)]),
                )
                .unwrap();

            assert_eq!(res, SupportedParameterOrReturnValue::Int(2));
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
                    Parameters(vec![
                        SupportedParameterOrReturnValue::Int(1),
                        SupportedParameterOrReturnValue::Int(2),
                    ]),
                )
                .unwrap();

            assert_eq!(res, SupportedParameterOrReturnValue::Int(3));
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

            let res = sandbox.call_host_function("test2", Parameters(vec![]));
            assert!(res.is_err());
        }

        // calling a function that doesn't exist
        {
            let usbox = uninitialized_sandbox();
            let sandbox = usbox.initialize(Some(init));
            assert!(sandbox.is_ok());
            let mut sandbox = sandbox.unwrap();

            let res = sandbox.call_host_function("test4", Parameters(vec![]));
            assert!(res.is_err());
        }
    }

    #[test]
    fn test_load_guest_binary_manual() {
        let cfg = SandboxMemoryConfiguration::default();

        let simple_guest_path = simple_guest_path().unwrap();
        let mgr =
            UnintializedSandbox::load_guest_binary(cfg, simple_guest_path.as_str(), false, false)
                .unwrap();
        assert_eq!(cfg, mgr.mem_cfg);
    }

    #[test]
    fn test_load_guest_binary_load_lib() {
        let cfg = SandboxMemoryConfiguration::default();
        let simple_guest_path = simple_guest_path().unwrap();
        let mgr_res =
            UnintializedSandbox::load_guest_binary(cfg, simple_guest_path.as_str(), true, true);
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

        let mut sandbox = UnintializedSandbox::new(
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

        let mut sandbox = UnintializedSandbox::new(
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
        let mut sandbox = UnintializedSandbox::new(
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

        let mut sandbox = UnintializedSandbox::new(
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

    fn new_guest_log_data(level: LogLevel) -> GuestLogData {
        GuestLogData::new(
            "test log".to_string(),
            "test source".to_string(),
            level,
            "test caller".to_string(),
            "test source file".to_string(),
            123,
        )
    }

    #[test]
    fn test_outb_log() {
        let new_mgr = || {
            let mut pe_info = simple_guest_pe_info().unwrap();
            SandboxMemoryManager::load_guest_binary_into_memory(
                SandboxMemoryConfiguration::default(),
                &mut pe_info,
                false,
            )
            .unwrap()
        };
        {
            // We have not set a logger and there is no guest log data
            // in memory, so expect a log operation to fail
            let mgr = new_mgr();
            assert!(outb_log(&mgr).is_err());
        }
        {
            // Write a log message so outb_log will succeed. Since there is
            // no logger set with set_logger, expect logs to be no-ops
            let mut mgr = new_mgr();
            let layout = mgr.layout;
            let log_msg = new_guest_log_data(LogLevel::Information);

            log_msg
                .write_to_memory(mgr.get_shared_mem_mut(), &layout)
                .unwrap();
            assert!(outb_log(&mgr).is_ok());
            assert_eq!(0, LOGGER.num_log_calls());
        }
        {
            // now, test logging
            let mut mgr = new_mgr();
            {
                // set up the logger and set the log level to the maximum
                // possible (Trace) to ensure we're able to test all
                // the possible branches of the match in outb_log
                set_logger(&LOGGER).unwrap();
                set_max_level(log::LevelFilter::Trace);
            }
            let levels = vec![
                LogLevel::Trace,
                LogLevel::Debug,
                LogLevel::Information,
                LogLevel::Warning,
                LogLevel::Error,
                LogLevel::Critical,
                LogLevel::None,
            ];
            for (idx, level) in levels.iter().enumerate() {
                let layout = mgr.layout;
                let log_data = new_guest_log_data(level.clone());
                log_data
                    .write_to_memory(mgr.get_shared_mem_mut(), &layout)
                    .unwrap();
                outb_log(&mgr).unwrap();
                let num_calls = LOGGER.num_log_calls();
                if level.clone() != LogLevel::None {
                    assert_eq!(
                        idx + 1,
                        num_calls,
                        "log call did not occur for level {:?}",
                        level.clone()
                    );
                }
                let last_log = LOGGER.get_log_call(num_calls - 1).unwrap();
                match (level, last_log.level) {
                    (LogLevel::Trace, Level::Trace) => (),
                    (LogLevel::Debug, Level::Debug) => (),
                    (LogLevel::Information, Level::Info) => (),
                    (LogLevel::Warning, Level::Warn) => (),
                    (LogLevel::Error, Level::Error) => (),
                    (LogLevel::Critical, Level::Error) => (),
                    // If someone logged with "None", we don't
                    // expect any actual log record. this case
                    // is here to indicate we don't want to
                    // match None to any actual log record, and
                    // we don't want to fall through to the next
                    // case
                    (LogLevel::None, _) => (),
                    (other_log_level, other_level) => panic!(
                        "Invalid LogLevel / Level pair: ({:?}, {:?})",
                        other_log_level, other_level
                    ),
                };
            }
        }
    }

    #[test]
    fn test_stack_guard() {
        let simple_guest_path = simple_guest_path().unwrap();
        let sbox = UnintializedSandbox::new(simple_guest_path, None, None).unwrap();
        let res = sbox.check_stack_guard();
        assert!(res.is_ok(), "Sandbox::check_stack_guard returned an error");
        assert!(res.unwrap(), "Sandbox::check_stack_guard returned false");
    }

    #[test]
    fn check_create_and_use_sandbox_on_different_threads() {
        let unintializedsandbox_queue = Arc::new(ArrayQueue::<UnintializedSandbox>::new(10));
        let sandbox_queue = Arc::new(ArrayQueue::<Sandbox>::new(10));

        for i in 0..10 {
            let simple_guest_path = simple_guest_path().expect("Guest Binary Missing");
            let unintializedsandbox = UnintializedSandbox::new(simple_guest_path, None, None)
                .unwrap_or_else(|_| panic!("Failed to create UnintializedSandbox {}", i));

            unintializedsandbox_queue
                .push(unintializedsandbox)
                .unwrap_or_else(|_| panic!("Failed to push UnintializedSandbox {}", i));
        }

        let thread_handles = (0..10)
            .map(|i| {
                let uq = unintializedsandbox_queue.clone();
                let sq = sandbox_queue.clone();
                thread::spawn(move || {
                    let mut uninitialized_sandbox = uq.pop().unwrap_or_else(|| {
                        panic!("Failed to pop UnintializedSandbox thread {}", i)
                    });
                    uninitialized_sandbox
                        .host_print(format!("Print from UnintializedSandbox on Thread {}\n", i))
                        .unwrap();

                    let sandbox = uninitialized_sandbox
                        .initialize::<fn(&mut UnintializedSandbox<'_>) -> Result<()>>(None)
                        .unwrap_or_else(|_| {
                            panic!("Failed to initialize UnintializedSandbox thread {}", i)
                        });

                    sq.push(sandbox).unwrap_or_else(|_| {
                        panic!("Failed to push UnintializedSandbox thread {}", i)
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
}
