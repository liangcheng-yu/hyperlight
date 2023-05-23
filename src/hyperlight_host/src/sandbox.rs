use anyhow::Result;
use std::collections::HashMap;
use std::option::Option;

use crate::func::{
    args::Val,
    def::{FuncCallError, GuestFunc, HostFunc},
};

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
}

impl Sandbox {
    /// Create a new sandbox configured to run the binary at path
    /// `bin_path`.
    pub fn new(bin_path: String) -> Self {
        Self {
            bin_path,
            host_funcs: HashMap::new(),
            guest_funcs: HashMap::new(),
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
}
