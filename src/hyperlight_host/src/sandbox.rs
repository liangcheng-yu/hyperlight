use super::sandbox_run_options::SandboxRunOptions;
use crate::guest_interface_glue::{HostMethodInfo, SupportedParameterAndReturnValues};
use crate::mem::ptr::RawPtr;
use crate::mem::{
    config::SandboxMemoryConfiguration, mgr::SandboxMemoryManager, pe::pe_info::PEInfo,
};
use anyhow::anyhow;
use anyhow::Result;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::io::stdout;
use std::io::Write;
use std::ops::Add;
use std::option::Option;
use std::path::Path;
use std::rc::Rc;
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

#[allow(unused)]
pub(crate) fn handle_outb(port: u16, byte: u8) -> Result<()> {
    match port.into() {
        OutBAction::Log => {
            // TODO
        }
        OutBAction::CallFunction => {
            // TODO
        }
        OutBAction::Abort => {
            // TODO
        }
        _ => {
            // TODO
        }
    }
    Ok(())
}

/// Determine whether a suitable hypervisor is available to run
/// this sandbox.
///
//  Returns a boolean indicating whether a suitable hypervisor is present.

// TODO - implement this
pub(crate) fn is_hypervisor_present() -> bool {
    #[cfg(target_os = "linux")]
    return true;
    #[cfg(target_os = "windows")]
    return true;
    #[cfg(not(target_os = "linux"))]
    #[cfg(not(target_os = "windows"))]
    false
}

/// The primary mechanism to interact with VM partitions that
/// run Hyperlight Sandboxes.
///
/// A Hyperlight Sandbox is a specialized VM environment
/// intended specifically for running Hyperlight guest processes.
pub struct Sandbox {
    // The writer to use for print requests from the guest.
    writer: Option<Rc<RefCell<dyn Write>>>,
    /// The map of host function names to their corresponding
    /// HostMethodInfo.
    map_host_function_names_to_method_info: HashMap<String, HostMethodInfo>,
    // The memory manager for the sandbox.
    mem_mgr: SandboxMemoryManager,
}

impl Sandbox {
    /// Create a new sandbox configured to run the binary at path
    /// `bin_path`.
    pub fn new(
        bin_path: String,
        cfg: Option<SandboxMemoryConfiguration>,
        writer: Option<Rc<RefCell<dyn Write>>>,
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
        let mem_mgr = Sandbox::load_guest_binary(
            mem_cfg,
            &bin_path,
            run_from_process_memory,
            run_from_guest_binary,
        )?;

        let this = Self {
            writer,
            mem_mgr,
            map_host_function_names_to_method_info: HashMap::new(),
        };

        // Register the host print function

        Ok(this)
    }

    /// Registers a host function onto map of host functions.
    ///
    /// Example usage:
    /// ```
    /// use hyperlight_host::guest_interface_glue::register_host_function;
    /// use hyperlight_host::guest::host_function_definition::{HostFunctionDefinition, ParamValueType, ReturnValueType};
    /// use hyperlight_host::guest_interface_glue::SupportedParameterAndReturnTypes;
    ///
    /// fn add(args: &[SupportedParameterAndReturnValues]) -> Result<SupportedParameterAndReturnValues> {
    ///    let a = match &args[0] {
    ///             SupportedParameterAndReturnValues::Int(a) => *a,
    ///             _ => return Err(anyhow!("Invalid type for a")),
    ///     };
    ///     let b = match &args[1] {
    ///             SupportedParameterAndReturnValues::Int(b) => *b,
    ///             _ => return Err(anyhow!("Invalid type for b")),
    ///     };
    ///     Ok(SupportedParameterAndReturnValues::Int(a + b))
    /// }
    ///
    ///
    /// fn main() {
    ///    let function = HostMethodInfo {
    ///       host_function_definition: HostFunctionDefinition {
    ///         function_name: "add".to_string(),
    ///         parameters: vec![ ParamValueType::Int, ParamValueType::Int ],
    ///         return_type: ReturnValueType::Int,
    ///       },
    ///       function_pointer: add,
    ///     };
    ///    register_host_function(function);
    /// ```
    ///
    pub fn register_host_function(&mut self, function: HostMethodInfo) -> Result<()> {
        let name = function.host_function_definition.function_name.to_string();
        let map = &mut self.map_host_function_names_to_method_info;

        // If already exists, replace
        if map.contains_key(&name) {
            // (DAN:TODO): log warning equiv. to "HyperlightLogger.LogWarning($"Updating MethodInfo for ${methodInfo.&Name} - there are multiple host methods with the same name.", GetType().Name);"
            map.remove(&name);
        }

        map.insert(name, function);
        Ok(())
    }

    /// Calls a host function by name.
    ///
    /// Example usage:
    /// ```
    /// // [...]
    /// match call_host_function("add", &vec![SupportedParameterAndReturnValues::Int(1), SupportedParameterAndReturnValues::Int(2)]) {
    ///     Ok(SupportedParameterAndReturnValues::Int(result)) => println!("Result: {}", result),
    ///     _ => println!("Invalid return type"),
    /// }
    /// // [...]
    /// ```
    ///
    pub fn call_host_function(
        &self,
        function_name: &str,
        args: &[SupportedParameterAndReturnValues],
    ) -> Result<SupportedParameterAndReturnValues> {
        let map = &self.map_host_function_names_to_method_info;

        let host_function = match map.get(function_name) {
            Some(host_function) => host_function,
            None => return Err(anyhow!("Host function not found")),
        };

        (host_function.function_pointer)(args)
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

    // TODO: once we have the host registration functionality we should remove this and hook it up in new()
    #[allow(unused)]
    fn host_print(&mut self, msg: &str) -> Result<()> {
        match &self.writer {
            Some(writer) => {
                writer.borrow_mut().write_all(msg.as_bytes())?;
                Ok(())
            }

            None => match atty::is(atty::Stream::Stdout) {
                false => {
                    stdout().write_all(msg.as_bytes())?;
                    Ok(())
                }
                true => {
                    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
                    let mut color_spec = ColorSpec::new();
                    color_spec.set_fg(Some(Color::Green));
                    stdout.set_color(&color_spec)?;
                    stdout.write_all(msg.as_bytes())?;
                    stdout.reset()?;
                    Ok(())
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sandbox;
    use crate::{
        mem::config::SandboxMemoryConfiguration, sandbox_run_options::SandboxRunOptions,
        testing::simple_guest_path,
    };
    use std::{cell::RefCell, io::Cursor, rc::Rc};
    #[test]

    fn test_new_sandbox() {
        // Guest Binary exists at path

        let binary_path = simple_guest_path().unwrap();
        let sandbox = Sandbox::new(binary_path.clone(), None, None, None);
        assert!(sandbox.is_ok());

        // Guest Binary does not exist at path

        let binary_path_does_not_exist = binary_path.trim_end_matches(".exe").to_string();
        let sandbox = Sandbox::new(binary_path_does_not_exist, None, None, None);
        assert!(sandbox.is_err());

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

        let sandbox = Sandbox::new(binary_path.clone(), Some(cfg), None, None);
        assert!(sandbox.is_ok());

        // Invalid sandbox_run_options

        let sandbox_run_options =
            SandboxRunOptions::RUN_FROM_GUEST_BINARY | SandboxRunOptions::RECYCLE_AFTER_RUN;

        let sandbox = Sandbox::new(binary_path, None, None, Some(sandbox_run_options));
        assert!(sandbox.is_err());
    }

    #[test]
    fn test_load_guest_binary_manual() {
        let cfg = SandboxMemoryConfiguration::default();

        let simple_guest_path = simple_guest_path().unwrap();
        let mgr =
            Sandbox::load_guest_binary(cfg, simple_guest_path.as_str(), false, false).unwrap();
        assert_eq!(cfg, mgr.mem_cfg);
    }

    #[test]
    fn test_load_guest_binary_load_lib() {
        let cfg = SandboxMemoryConfiguration::default();
        let simple_guest_path = simple_guest_path().unwrap();
        let mgr_res = Sandbox::load_guest_binary(cfg, simple_guest_path.as_str(), true, true);
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
        // Test with a writer

        let cursor = Cursor::new(vec![0; 4]);
        let writer = Rc::new(RefCell::new(cursor));
        let mut sandbox = Sandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            Some(writer.clone()),
            None,
        )
        .expect("Failed to create sandbox");

        sandbox.host_print("test").unwrap();

        let ref_writer = writer.borrow();
        let buffer = ref_writer.get_ref();
        assert_eq!(buffer, b"test");

        // TODO: Test with stdout
    }
}
