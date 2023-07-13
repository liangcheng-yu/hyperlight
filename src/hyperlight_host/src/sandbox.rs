use super::sandbox_run_options::SandboxRunOptions;
use crate::flatbuffers::hyperlight::generated::ErrorCode;
use crate::func::function_types::{ParameterValue, ReturnValue};
use crate::func::guest::log_data::GuestLogData;
use crate::func::host::function_definition::HostFunctionDefinition;
use crate::func::host::{Function1, HyperlightFunction};
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
use is_terminal::IsTerminal;
use log::{error, info, warn, Level, Record};
use std::collections::HashMap;
use std::ffi::c_void;
use std::io::stdout;
use std::io::Write;
use std::ops::Add;
use std::option::Option;
use std::path::Path;
use std::sync::{Arc, Mutex};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tracing::instrument;
use tracing_log::format_trace;
use uuid::Uuid;

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
    correlation_id: String,
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

impl<'a> crate::sandbox_state::sandbox::Sandbox for Sandbox<'a> {}

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

impl<'a> crate::sandbox_state::sandbox::Sandbox for UnintializedSandbox<'a> {}

impl<'a, F>
    EvolvableSandbox<
        UnintializedSandbox<'a>,
        Sandbox<'a>,
        MutatingCallback<'a, UnintializedSandbox<'a>, F>,
    > for UnintializedSandbox<'a>
where
    F: FnOnce(&mut UnintializedSandbox<'a>) -> Result<()> + 'a,
{
    /// Evolve `self` into a `Sandbox`, executing a caller-provided
    /// callback during the transition process.
    ///
    /// If you need to do this transition without a callback, use the
    /// `EvolvableSandbox` implementation that takes a `Noop`.
    fn evolve(mut self, tsn: MutatingCallback<UnintializedSandbox<'a>, F>) -> Result<Sandbox<'a>> {
        tsn.call(&mut self)?;
        // TODO: snapshot memory here so we can take the returned
        // Sandbox and revert back to an UninitializedSandbox
        Ok(Sandbox {
            host_functions: self.host_functions.clone(),
            mem_mgr: self.mem_mgr.clone(),
            stack_guard: self.stack_guard,
            uninit_sandbox: self,
        })
    }
}

impl<'a>
    EvolvableSandbox<
        UnintializedSandbox<'a>,
        Sandbox<'a>,
        Noop<UnintializedSandbox<'a>, Sandbox<'a>>,
    > for UnintializedSandbox<'a>
{
    /// Evolve `self` to a `Sandbox` without any additional metadata.
    ///
    /// If you want to pass a callback to this state transition so you can
    /// run your own code during the transition, use the `EvolvableSandbox`
    /// implementation that accepts a `MutatingCallback`
    fn evolve(self, _: Noop<UnintializedSandbox<'a>, Sandbox<'a>>) -> Result<Sandbox<'a>> {
        // TODO: snapshot memory here so we can take the returned
        // Sandbox and revert back to an UninitializedSandbox
        Ok(Sandbox {
            host_functions: self.host_functions.clone(),
            mem_mgr: self.mem_mgr.clone(),
            stack_guard: self.stack_guard,
            uninit_sandbox: self,
        })
    }
}

impl<'a> UnintializedSandbox<'a> {
    /// Create a new sandbox configured to run the binary at path
    /// `bin_path`.
    ///
    /// The instrument attribute is used to generate tracing spans and also to emit an error should the Result be an error
    /// In order to ensure that the span is associated with any error (so we get the correlation id ) we set the level of the span to error
    /// the downside to this is that if there is no trace subscriber a log at level error or below will always emit a record for this regardless of if an error actually occurs
    /// TODO: Move this to C API and just leave   #[instrument(err(Dubug))] on the function
    #[instrument(
        err(),
        skip(cfg),
        fields(correlation_id)
        name = "UnintializedSandbox::new"
    )]
    pub fn new(
        bin_path: String,
        cfg: Option<SandboxMemoryConfiguration>,
        sandbox_run_options: Option<SandboxRunOptions>,
        correlation_id: Option<String>,
    ) -> Result<Self> {
        let correlation_id = match correlation_id {
            None => {
                info!("No correlation id provided, generating one");
                Uuid::new_v4().to_string()
            }
            Some(id) => {
                info!("Using provided correlation id");
                id
            }
        };

        tracing::Span::current().record("correlation_id", &correlation_id);

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
            match stdout().is_terminal() {
                false => {
                    print!("{}", s);
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
            correlation_id,
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
    #[instrument(err(Debug), skip(self))]
    fn check_stack_guard(&self) -> Result<bool> {
        self.mem_mgr.check_stack_guard(self.stack_guard)
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
    #[instrument(err(Debug), skip_all, fields(correlation_id=self.correlation_id))]
    fn initialize<F: Fn(&mut UnintializedSandbox<'a>) -> Result<()> + 'a>(
        mut self,
        callback: Option<F>,
    ) -> Result<Sandbox<'a>> {
        match callback {
            Some(cb) => self.evolve(MutatingCallback::from(cb)),
            None => self.evolve(Noop::default()),
        }
    }

    /// Host print function – an exception to normal calling functions
    /// as it can be called prior to initialization.
    pub(crate) fn host_print(&mut self, msg: String) -> Result<()> {
        let writer_func = self
            .host_functions
            .get_mut("writer_func")
            .ok_or_else(|| anyhow!("Host function 'writer_func' not found"))?;

        let mut writer_locked_func = writer_func
            .lock()
            .map_err(|e| anyhow!("error locking: {:?}", e))?;
        writer_locked_func(vec![ParameterValue::String(msg)])?;

        Ok(())
    }
}

impl<'a> Sandbox<'a> {
    /// Call a host print in the sandbox.
    #[allow(unused)]
    pub(crate) fn host_print(&mut self, msg: String) -> Result<()> {
        self.call_host_function("writer_func", vec![ParameterValue::String(msg)])?;

        Ok(())
    }

    /// Call a host function in the sandbox.
    pub fn call_host_function(
        &mut self,
        name: &str,
        args: Vec<ParameterValue>,
    ) -> Result<ReturnValue> {
        let func = self
            .host_functions
            .get(name)
            .ok_or_else(|| anyhow!("Host function {} not found", name))?;

        let mut locked_func = func.lock().map_err(|e| anyhow!("error locking: {:?}", e))?;
        locked_func(args)
    }

    #[allow(unused)]
    pub(crate) fn handle_outb(&mut self, port: u16, byte: u8) -> Result<()> {
        match port.into() {
            OutBAction::Log => outb_log(&self.mem_mgr),
            OutBAction::CallFunction => {
                let call = self.mem_mgr.get_host_function_call()?;
                let name = call.function_name.clone();
                let args: Vec<ParameterValue> = call.parameters.clone().unwrap_or(vec![]);
                let res = self.call_host_function(&name, args)?;
                self.mem_mgr.write_response_from_host_method_call(&res)?;
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
#[instrument(skip(mgr))]
fn outb_log(mgr: &SandboxMemoryManager) -> Result<()> {
    // This code will create either a logging record or a tracing record for the GuestLogData depending on if the host has set up a tracing subscriber.
    // In theory as we have enabled the log feature in the Cargo.toml for tracing this should happen
    // automatically (based on if there is tracing subscriber present) but only works if the event created using macros. (see https://github.com/tokio-rs/tracing/blob/master/tracing/src/macros.rs#L2421 )
    // The reason that we don't want to use the tracing macros is that we want to be able to explicitly
    // set the file and line number for the log record which is not possible with macros.
    // This is because the file and line number come from the  guest not the call site.

    let log_data: GuestLogData = mgr.read_guest_log_data()?;

    let record_level: &Level = &log_data.level.into();

    // Work out if we need to log or trace
    // this API is marked as follows but it is the easiest way to work out if we should trace or log

    // Private API for internal use by tracing's macros.
    //
    // This function is *not* considered part of `tracing`'s public API, and has no
    // stability guarantees. If you use it, and it breaks or disappears entirely,
    // don't say we didn't warn you.

    let should_trace = tracing_core::dispatcher::has_been_set();
    let source_file = Some(log_data.source_file.as_str());
    let line = Some(log_data.line);
    let source = Some(log_data.source.as_str());

    // See https://github.com/rust-lang/rust/issues/42253 for the reason this has to be done this way

    if should_trace {
        // Create a tracing event for the GuestLogData
        // Ideally we would create tracing metadata based on the Guest Log Data
        // but tracing derives the metadata at compile time
        // see https://github.com/tokio-rs/tracing/issues/2419
        // so we leave it up to the subscriber to figure out that there are logging fields present with this data
        format_trace(
            &Record::builder()
                .args(format_args!("{}", log_data.message))
                .level(*record_level)
                .target("hyperlight_guest")
                .file(source_file)
                .line(line)
                .module_path(source)
                .build(),
        )?;
    } else {
        // Create a log record for the GuestLogData
        log::logger().log(
            &Record::builder()
                .args(format_args!("{}", log_data.message))
                .level(*record_level)
                .target("hyperlight_guest")
                .file(Some(&log_data.source_file))
                .line(Some(log_data.line))
                .module_path(Some(&log_data.source))
                .build(),
        );
    }

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
        func::guest::{log_data::GuestLogData, log_level::LogLevel},
        func::{
            function_types::{ParameterValue, ReturnValue},
            host::{Function1, Function2},
        },
        mem::{config::SandboxMemoryConfiguration, mgr::SandboxMemoryManager},
        sandbox_run_options::SandboxRunOptions,
        testing::{
            logger::Logger as TestLogger, logger::LOGGER as TEST_LOGGER, simple_guest_path,
            simple_guest_pe_info, tracing_subscriber::TracingSubscriber as TestSubcriber,
        },
    };
    use anyhow::Result;
    use crossbeam_queue::ArrayQueue;
    use log::Level;
    #[cfg(not(RunningNextest))]
    use serial_test::serial;
    use std::path::PathBuf;
    use std::{
        io::{Read, Write},
        sync::{Arc, Mutex},
        thread,
    };
    use tempfile::NamedTempFile;
    use tracing::Level as tracing_level;
    use tracing_core::{callsite::rebuild_interest_cache, Subscriber};
    use uuid::Uuid;
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
    use serde_json::{Map, Value};

    #[test]
    fn test_new_sandbox() {
        // Guest Binary exists at path

        let binary_path = simple_guest_path().unwrap();
        let sandbox = UnintializedSandbox::new(binary_path.clone(), None, None, None);
        assert!(sandbox.is_ok());

        // Guest Binary does not exist at path

        let binary_path_does_not_exist = binary_path.trim_end_matches(".exe").to_string();
        let uninitialized_sandbox =
            UnintializedSandbox::new(binary_path_does_not_exist, None, None, None);
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

        let uninitialized_sandbox =
            UnintializedSandbox::new(binary_path.clone(), Some(cfg), None, None);
        assert!(uninitialized_sandbox.is_ok());

        // Invalid sandbox_run_options

        let sandbox_run_options =
            SandboxRunOptions::RUN_FROM_GUEST_BINARY | SandboxRunOptions::RECYCLE_AFTER_RUN;

        let uninitialized_sandbox =
            UnintializedSandbox::new(binary_path.clone(), None, Some(sandbox_run_options), None);
        assert!(uninitialized_sandbox.is_err());

        let uninitialized_sandbox = UnintializedSandbox::new(binary_path, None, None, None);
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
    fn test_load_guest_binary_manual() {
        let cfg = SandboxMemoryConfiguration::default();

        let simple_guest_path = simple_guest_path().unwrap();
        let mgr =
            UnintializedSandbox::load_guest_binary(cfg, simple_guest_path.as_str(), false, false)
                .unwrap();
        assert_eq!(cfg, mgr.mem_cfg);
    }

    #[test]
    #[cfg_attr(not(RunningNextest), serial)]
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
    fn test_log_outb_log() {
        TestLogger::initialize_test_logger();
        TEST_LOGGER.set_max_level(log::LevelFilter::Off);

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
            // We set a logger but there is no guest log data
            // in memory, so expect a log operation to fail
            let mgr = new_mgr();
            assert!(outb_log(&mgr).is_err());
        }
        {
            // Write a log message so outb_log will succeed.
            // Since the logger level is set off, expect logs to be no-ops
            let mut mgr = new_mgr();
            let layout = mgr.layout;
            let log_msg = new_guest_log_data(LogLevel::Information);

            log_msg
                .write_to_memory(mgr.get_shared_mem_mut(), &layout)
                .unwrap();
            assert!(outb_log(&mgr).is_ok());
            assert_eq!(0, TEST_LOGGER.num_log_calls());
            TEST_LOGGER.clear_log_calls();
        }
        {
            // now, test logging
            TEST_LOGGER.set_max_level(log::LevelFilter::Trace);
            let mut mgr = new_mgr();
            TEST_LOGGER.clear_log_calls();

            // set up the logger and set the log level to the maximum
            // possible (Trace) to ensure we're able to test all
            // the possible branches of the match in outb_log

            let levels = vec![
                LogLevel::Trace,
                LogLevel::Debug,
                LogLevel::Information,
                LogLevel::Warning,
                LogLevel::Error,
                LogLevel::Critical,
                LogLevel::None,
            ];
            for level in levels {
                let layout = mgr.layout;
                let log_data = new_guest_log_data(level);
                log_data
                    .write_to_memory(mgr.get_shared_mem_mut(), &layout)
                    .unwrap();
                outb_log(&mgr).unwrap();

                TEST_LOGGER.test_log_records(|log_calls| {
                    let expected_level: Level = level.into();

                    assert!(
                        log_calls
                            .iter()
                            .filter(|log_call| {
                                log_call.level == expected_level
                                    && log_call.line == Some(log_data.line)
                                    && log_call.args == log_data.message
                                    && log_call.module_path == Some(log_data.source.clone())
                                    && log_call.file == Some(log_data.source_file.clone())
                            })
                            .count()
                            == 1,
                        "log call did not occur for level {:?}",
                        level.clone()
                    );
                });
            }
        }
    }

    // Tests that outb_log emits traces when a trace subscriber is set
    // this test is ignored because it is incompatible with other tests , specifically those which require a logger for tracing
    // to run tracing tests use `cargo test test_trace -- --ignored`
    #[test]
    #[ignore]
    #[cfg_attr(not(RunningNextest), serial)]
    fn test_trace_outb_log() {
        TestLogger::initialize_log_tracer();
        rebuild_interest_cache();
        let subscriber = TestSubcriber::new(tracing_level::TRACE);
        tracing::subscriber::with_default(subscriber.clone(), || {
            let new_mgr = || {
                let mut pe_info = simple_guest_pe_info().unwrap();
                SandboxMemoryManager::load_guest_binary_into_memory(
                    SandboxMemoryConfiguration::default(),
                    &mut pe_info,
                    false,
                )
                .unwrap()
            };

            // as a span does not exist one will be automatically created
            // after that there will be an event for each log message
            // we are interested only in the events for the log messages that we created

            let levels = vec![
                LogLevel::Trace,
                LogLevel::Debug,
                LogLevel::Information,
                LogLevel::Warning,
                LogLevel::Error,
                LogLevel::Critical,
                LogLevel::None,
            ];
            for level in levels {
                let mut mgr = new_mgr();
                let layout = mgr.layout;
                let log_data: GuestLogData = new_guest_log_data(level);
                log_data
                    .write_to_memory(mgr.get_shared_mem_mut(), &layout)
                    .unwrap();
                outb_log(&mgr).unwrap();

                subscriber.test_trace_records(|spans, events| {
                    let expected_level = match level {
                        LogLevel::Trace => "TRACE",
                        LogLevel::Debug => "DEBUG",
                        LogLevel::Information => "INFO",
                        LogLevel::Warning => "WARN",
                        LogLevel::Error => "ERROR",
                        LogLevel::Critical => "ERROR",
                        LogLevel::None => "TRACE",
                    };

                    // We cannot get the span using the `current_span()` method as by the time we get to this point the span has been exited so there is no current span
                    // We need to make sure that the span that we created is in the spans map instead
                    // We should only have one span in the map

                    assert!(spans.len() == 1);

                    let span_value = spans
                        .get(&1)
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .get("span")
                        .unwrap()
                        .get("attributes")
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .get("metadata")
                        .unwrap()
                        .as_object()
                        .unwrap();

                    assert!(test_value_as_str(span_value, "level", "INFO"));
                    assert!(test_value_as_str(
                        span_value,
                        "module_path",
                        "hyperlight_host::sandbox"
                    ));
                    let expected_file = if cfg!(windows) {
                        "src\\hyperlight_host\\src\\sandbox.rs"
                    } else {
                        "src/hyperlight_host/src/sandbox.rs"
                    };
                    assert!(test_value_as_str(span_value, "file", expected_file));
                    assert!(test_value_as_str(
                        span_value,
                        "target",
                        "hyperlight_host::sandbox"
                    ));

                    let mut count_matching_events = 0;

                    for json_value in events {
                        let event_values = json_value.as_object().unwrap().get("event").unwrap();
                        let metadata_values_map =
                            event_values.get("metadata").unwrap().as_object().unwrap();
                        let event_values_map = event_values.as_object().unwrap();
                        if test_value_as_str(metadata_values_map, "level", expected_level)
                            && test_value_as_str(event_values_map, "log.file", "test source file")
                            && test_value_as_str(event_values_map, "log.module_path", "test source")
                            && test_value_as_str(event_values_map, "log.target", "hyperlight_guest")
                        {
                            count_matching_events += 1;
                        }
                    }
                    assert!(
                        count_matching_events == 1,
                        "trace log call did not occur for level {:?}",
                        level.clone()
                    );
                    subscriber.clear();
                });
            }
        });
    }

    fn test_value_as_str(values: &Map<String, Value>, key: &str, expected_value: &str) -> bool {
        if let Some(value) = values.get(key) {
            if let Some(value) = value.as_str() {
                if value == expected_value {
                    return true;
                }
            }
        };
        false
    }

    fn test_value_as_str_starts_with(
        values: &Map<String, Value>,
        key: &str,
        expected_value: &str,
    ) -> bool {
        if let Some(value) = values.get(key) {
            if let Some(value) = value.as_str() {
                if value.starts_with(expected_value) {
                    return true;
                }
            }
        };
        false
    }

    #[test]
    fn test_stack_guard() {
        let simple_guest_path = simple_guest_path().unwrap();
        let sbox = UnintializedSandbox::new(simple_guest_path, None, None, None).unwrap();
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
            let unintializedsandbox = UnintializedSandbox::new(simple_guest_path, None, None, None)
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

    #[test]
    #[ignore]
    #[cfg_attr(not(RunningNextest), serial)]
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

            assert!(test_value_as_str(
                span_attributes,
                "correlation_id",
                correlation_id.as_str()
            ));

            let mut binary_path = simple_guest_path().unwrap();
            binary_path.push_str("does_not_exist");

            let correlation_id = Uuid::new_v4().as_hyphenated().to_string();
            let sbox =
                UnintializedSandbox::new(binary_path, None, None, Some(correlation_id.clone()));
            assert!(sbox.is_err());

            // Now we should still be in span 1 but span 2 should be created (we created entered and exited span 2 when we called UnintializedSandbox::new)

            let current_span = subscriber.current_span();
            assert!(current_span.is_known(), "Current span is unknown");
            let current_span_metadata = current_span.into_inner().unwrap();
            assert_eq!(
                current_span_metadata.0.into_u64(),
                1,
                "Current span is not span 1"
            );

            let span_metadata = subscriber.get_span_metadata(2);
            assert_eq!(span_metadata.name(), "UnintializedSandbox::new");

            // The value of the correlation id should be the same as the one we passed to UnintializedSandbox::new

            let span_data = subscriber.get_span(2);
            let span_attributes: &Map<String, Value> = span_data
                .get("span")
                .unwrap()
                .get("attributes")
                .unwrap()
                .as_object()
                .unwrap();

            assert!(test_value_as_str(
                span_attributes,
                "correlation_id",
                correlation_id.as_str()
            ));

            // There should be two events, one for the info specifying that the provided correlation id is being used and the other for the error that the binary path does not exist

            let events = subscriber.get_events();
            assert_eq!(events.len(), 2);

            let mut count_matching_events = 0;

            for json_value in events {
                let event_values = json_value.as_object().unwrap().get("event").unwrap();
                let metadata_values_map =
                    event_values.get("metadata").unwrap().as_object().unwrap();
                let event_values_map = event_values.as_object().unwrap();

                // This is the info event for using the provided correlation id

                if test_value_as_str(metadata_values_map, "level", "INFO")
                    && test_value_as_str(
                        event_values_map,
                        "message",
                        "Using provided correlation id",
                    )
                    && test_value_as_str(
                        event_values_map,
                        "log.module_path",
                        "hyperlight_host::sandbox",
                    )
                    && test_value_as_str(event_values_map, "log.target", "hyperlight_host::sandbox")
                {
                    count_matching_events += 1;
                }

                // This is the error event for the binary path not existing

                #[cfg(target_os = "windows")]
                let expected_error =
                    "Error The system cannot find the file specified. (os error 2) File Path";
                #[cfg(not(target_os = "windows"))]
                let expected_error = "Error No such file or directory (os error 2) File Path";

                if test_value_as_str(metadata_values_map, "level", "ERROR")
                    && test_value_as_str_starts_with(event_values_map, "error", expected_error)
                    && test_value_as_str(
                        metadata_values_map,
                        "module_path",
                        "hyperlight_host::sandbox",
                    )
                    && test_value_as_str(metadata_values_map, "target", "hyperlight_host::sandbox")
                {
                    count_matching_events += 1;
                }
            }
            assert!(
                count_matching_events == 2,
                "Unexpected number of matching events {}",
                count_matching_events
            );
            span.exit();
            subscriber.clear();
        });
    }

    #[test]
    // Tests that traces are emitted as log records when there is no trace subscriber configured.
    fn test_log_trace() {
        TestLogger::initialize_test_logger();
        TEST_LOGGER.set_max_level(log::LevelFilter::Trace);

        // This makes sure that the metadata interest cache is rebuilt so that the log records are emitted for the trace records

        rebuild_interest_cache();

        let mut binary_path = simple_guest_path().unwrap();
        binary_path.push_str("does_not_exist");

        let sbox = UnintializedSandbox::new(binary_path, None, None, None);
        assert!(sbox.is_err());

        // When tracng is creating log records it will create a log record for the creation of the span (from the instrument attribute), and will then create a log record for the entry to and exit from the span.
        // It also creates a log record for the span beign dropped.
        // So we expect 6 log records for this test, four for the span and then two for the error as the file that we are attempting to load into the sandbxo does not exist

        let num_calls = TEST_LOGGER.num_log_calls();
        assert_eq!(6, num_calls);

        // Log record 1

        let logcall = TEST_LOGGER.get_log_call(0).unwrap();
        assert_eq!(Level::Info, logcall.level);

        assert!(logcall
            .args
            .starts_with("UnintializedSandbox::new; bin_path"));
        assert_eq!("hyperlight_host::sandbox", logcall.target);

        // Log record 2

        let logcall = TEST_LOGGER.get_log_call(1).unwrap();
        assert_eq!(Level::Trace, logcall.level);
        assert_eq!(logcall.args, "-> UnintializedSandbox::new;");
        assert_eq!("tracing::span::active", logcall.target);

        // Log record 3

        let logcall = TEST_LOGGER.get_log_call(2).unwrap();
        assert_eq!(Level::Info, logcall.level);
        assert_eq!("No correlation id provided, generating one", logcall.args);
        assert_eq!("hyperlight_host::sandbox", logcall.target);

        // Log record 4

        let logcall = TEST_LOGGER.get_log_call(3).unwrap();
        assert_eq!(Level::Error, logcall.level);
        #[cfg(target_os = "windows")]
        assert!(logcall.args.starts_with(
            "error=Error The system cannot find the file specified. (os error 2) File Path"
        ));
        #[cfg(not(target_os = "windows"))]
        assert!(logcall
            .args
            .starts_with("error=Error No such file or directory (os error 2) File Path"));
        assert_eq!("hyperlight_host::sandbox", logcall.target);

        // Log record 5

        let logcall = TEST_LOGGER.get_log_call(4).unwrap();
        assert_eq!(Level::Trace, logcall.level);
        assert_eq!(logcall.args, "<- UnintializedSandbox::new;");
        assert_eq!("tracing::span::active", logcall.target);

        // Log record 6

        let logcall = TEST_LOGGER.get_log_call(5).unwrap();
        assert_eq!(Level::Trace, logcall.level);
        assert_eq!(logcall.args, "-- UnintializedSandbox::new;");
        assert_eq!("tracing::span", logcall.target);

        TEST_LOGGER.clear_log_calls();
        TEST_LOGGER.set_max_level(log::LevelFilter::Info);

        let mut invalid_binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        invalid_binary_path.push("src");
        invalid_binary_path.push("sandbox.rs");

        let sbox = UnintializedSandbox::new(
            invalid_binary_path.into_os_string().into_string().unwrap(),
            None,
            None,
            None,
        );
        assert!(sbox.is_err());

        // There should be six calls again as we changed the log LevelFilter to Info
        // We should see the 2 info level logs  seen in records 1 and 3 above
        // We should then see the span and the info log record from pe_info
        // and then finally the 2 errors from pe info and sandbox as the error result is propagated back up the call stack

        let num_calls = TEST_LOGGER.num_log_calls();
        assert_eq!(6, num_calls);

        // Log record 1

        let logcall = TEST_LOGGER.get_log_call(0).unwrap();
        assert_eq!(Level::Info, logcall.level);

        assert!(logcall
            .args
            .starts_with("UnintializedSandbox::new; bin_path"));
        assert_eq!("hyperlight_host::sandbox", logcall.target);

        // Log record 2

        let logcall = TEST_LOGGER.get_log_call(1).unwrap();
        assert_eq!(Level::Info, logcall.level);
        assert_eq!("No correlation id provided, generating one", logcall.args);
        assert_eq!("hyperlight_host::sandbox", logcall.target);

        // Log record 3

        let logcall = TEST_LOGGER.get_log_call(2).unwrap();
        assert_eq!(Level::Info, logcall.level);
        assert!(logcall.args.starts_with("from_file; filename="));
        assert_eq!("hyperlight_host::mem::pe::pe_info", logcall.target);

        // Log record 4

        let logcall = TEST_LOGGER.get_log_call(3).unwrap();
        assert_eq!(Level::Info, logcall.level);
        assert!(logcall.args.starts_with("Loading PE file from"));
        assert_eq!("hyperlight_host::mem::pe::pe_info", logcall.target);

        // Log record 5

        let logcall = TEST_LOGGER.get_log_call(4).unwrap();
        assert_eq!(Level::Error, logcall.level);
        assert!(logcall
            .args
            .starts_with("error=Malformed entity: DOS header is malformed"));
        assert_eq!("hyperlight_host::mem::pe::pe_info", logcall.target);

        // Log record 6

        let logcall = TEST_LOGGER.get_log_call(5).unwrap();
        assert_eq!(Level::Error, logcall.level);
        assert!(logcall
            .args
            .starts_with("error=Malformed entity: DOS header is malformed"));
        assert_eq!("hyperlight_host::sandbox", logcall.target);

        TEST_LOGGER.clear_log_calls();
        TEST_LOGGER.set_max_level(log::LevelFilter::Error);

        // Now we have set the max level to error, so we should not see any log calls as the following should not create an error

        let sbox = UnintializedSandbox::new(simple_guest_path().unwrap(), None, None, None);

        let sbox = sbox.unwrap();
        let _ = sbox.initialize::<fn(&mut UnintializedSandbox<'_>) -> Result<()>>(None);

        let num_calls = TEST_LOGGER.num_log_calls();
        assert_eq!(0, num_calls);
    }
}
