use super::host_funcs::{CallHostFunction, HostFunctionsMap};
use super::uninitialized::UninitializedSandbox;
use super::{host_funcs::CallHostPrint, outb::OutBAction};
use super::{host_funcs::HostFuncs, outb::outb_log};
use crate::func::function_types::ParameterValue;
use crate::func::host::HyperlightFunction;
use crate::mem::mgr::SandboxMemoryManager;
use crate::mem::mgr::STACK_COOKIE_LEN;
use anyhow::Result;
use std::collections::HashMap;

/// The primary mechanism to interact with VM partitions that run Hyperlight
/// guest binaries.
///
/// These can't be created directly. You must first create an
/// `UninitializedSandbox`, and then call `evolve` or `initialize` on it to
/// generate one of these.
#[allow(unused)]
pub struct Sandbox<'a> {
    // Registered host functions
    host_functions: HashMap<String, HyperlightFunction<'a>>,
    // The memory manager for the sandbox.
    mem_mgr: SandboxMemoryManager,
    stack_guard: [u8; STACK_COOKIE_LEN],
}

impl<'a> From<UninitializedSandbox<'a>> for Sandbox<'a> {
    fn from(val: UninitializedSandbox<'a>) -> Self {
        Self {
            host_functions: val.host_functions.clone(),
            mem_mgr: val.mem_mgr.clone(),
            stack_guard: val.stack_guard,
        }
    }
}

impl<'a> HostFuncs<'a> for Sandbox<'a> {
    fn get_host_funcs(&self) -> &HostFunctionsMap<'a> {
        &self.host_functions
    }
}

impl<'a> CallHostFunction<'a> for Sandbox<'a> {}

impl<'a> CallHostPrint<'a> for Sandbox<'a> {}

impl<'a> crate::sandbox_state::sandbox::Sandbox for Sandbox<'a> {}

impl<'a> std::fmt::Debug for Sandbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sandbox")
            .field("stack_guard", &self.stack_guard)
            .field("num_host_funcs", &self.host_functions.len())
            .finish()
    }
}

impl<'a> Sandbox<'a> {
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
