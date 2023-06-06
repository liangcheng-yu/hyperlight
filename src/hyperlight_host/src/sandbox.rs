use crate::mem::ptr::RawPtr;
use crate::{
    func::{
        args::Val,
        def::{FuncCallError, GuestFunc, HostFunc},
    },
    mem::{config::SandboxMemoryConfiguration, mgr::SandboxMemoryManager, pe::pe_info::PEInfo},
};
use anyhow::anyhow;
use anyhow::Result;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ops::Add;
use std::option::Option;

// In case its not obvious why there are separate is_supported_platform and is_hypervisor_present functions its because
// Hyerplight is designed to be able to run on a host that doesn't have a hypervisor.
// In that case, the sandbox will be in porcess, we plan on making this a dev only feature and fixing up Linux support
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
    /// The path to the binary that will be executed in the sandbox.
    pub bin_path: String,
    /// The functions to be available to the guest but are implemented
    /// on the host side.
    pub host_funcs: HashMap<String, HostFunc>,
    /// The functions that are implemented within the guest and are
    /// callable by the host.
    pub guest_funcs: HashMap<String, GuestFunc>,
    mem_mgr: SandboxMemoryManager,
}

impl Sandbox {
    /// Create a new sandbox configured to run the binary at path
    /// `bin_path`.
    pub fn new(bin_path: String, mgr: SandboxMemoryManager) -> Self {
        Self {
            bin_path,
            host_funcs: HashMap::new(),
            guest_funcs: HashMap::new(),
            mem_mgr: mgr,
        }
    }

    /// registers a function to be available to the
    /// host but implemented in the guest.
    /// Returns None if the function didn't already
    /// exist, and Some if it did. The value inside the
    /// Some will be the old value
    pub fn register_guest_func(&mut self, func: GuestFunc) -> Option<GuestFunc> {
        self.guest_funcs.insert(func.name.clone(), func)
    }

    /// registers a function to be available to the guest,
    /// but implemented inside the host
    pub fn register_host_func(&mut self, name: String, func_def: HostFunc) -> Option<HostFunc> {
        self.host_funcs.insert(name, func_def)
    }

    /// make a call from host to the guest function
    /// and return either its raw return value or an error
    pub fn call_guest_func(&self, func_name: String, args: &Val) -> Result<Val, FuncCallError> {
        self.guest_funcs
            .get(&func_name)
            .ok_or(FuncCallError {
                message: format!("Function {} not found", func_name),
            })?
            .call(args)
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
    pub(crate) fn load_guest_binary(
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
}

#[cfg(test)]
mod tests {
    use super::Sandbox;
    use crate::{mem::config::SandboxMemoryConfiguration, testing::simple_guest_path};

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
}
