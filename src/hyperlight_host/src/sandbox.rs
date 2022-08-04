use anyhow::Result;
use std::collections::HashMap;
use std::option::Option;

use crate::func::{
    args::Val,
    def::{FuncCallError, GuestFunc, HostFunc},
};

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

    /// Determine whether a suitable hypervisor is available to run
    /// this sandbox.
    ///
    /// Returns `Ok` with a boolean if it could be determined whether
    /// an appropriate hypervisor is available, and `Err` otherwise.
    pub fn is_hypervisor_present(&self) -> Result<bool> {
        // TODO: implement
        Ok(true)
    }
}
