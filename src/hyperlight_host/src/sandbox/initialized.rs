use super::guest_funcs::CallGuestFunction;
use super::guest_mgr::GuestMgr;
use super::uninitialized::UninitializedSandbox;
use super::{host_funcs::CallHostPrint, outb::OutBAction};
use super::{host_funcs::HostFuncs, outb::outb_log};
use super::{
    host_funcs::{CallHostFunction, HostFunctionsMap},
    mem_mgr::MemMgr,
};
use crate::flatbuffers::hyperlight::generated::ErrorCode;
use crate::func::function_call::{FunctionCall, FunctionCallType};
use crate::func::types::{ParameterValue, ReturnType};
use crate::mem::mgr::SandboxMemoryManager;
use crate::mem::mgr::STACK_COOKIE_LEN;
use crate::sandbox_state::reset::RestoreSandbox;
use anyhow::{bail, Result};
use log::error;
use std::sync::atomic::AtomicI32;

/// The primary mechanism to interact with VM partitions that run Hyperlight
/// guest binaries.
///
/// These can't be created directly. You must first create an
/// `UninitializedSandbox`, and then call `evolve` or `initialize` on it to
/// generate one of these.
#[allow(unused)]
pub struct Sandbox<'a> {
    // Registered host functions
    host_functions: HostFunctionsMap<'a>,
    // The memory manager for the sandbox.
    mem_mgr: SandboxMemoryManager,
    stack_guard: [u8; STACK_COOKIE_LEN],
    executing_guest_call: AtomicI32,
    needs_state_reset: bool,
    num_runs: i32,
}

impl<'a> crate::sandbox_state::sandbox::InitializedSandbox<'a> for Sandbox<'a> {
    fn get_initialized_sandbox(&self) -> &crate::sandbox::Sandbox<'a> {
        self
    }

    fn get_initialized_sandbox_mut(&mut self) -> &mut crate::sandbox::Sandbox<'a> {
        self
    }
}

impl<'a> From<UninitializedSandbox<'a>> for Sandbox<'a> {
    fn from(val: UninitializedSandbox<'a>) -> Self {
        Self {
            host_functions: val.get_host_funcs().clone(),
            mem_mgr: val.get_mem_mgr().clone(),
            stack_guard: *val.get_stack_cookie(),
            executing_guest_call: AtomicI32::new(0),
            needs_state_reset: false,
            num_runs: 0,
        }
    }
}

impl<'a> HostFuncs<'a> for Sandbox<'a> {
    fn get_host_funcs(&self) -> &HostFunctionsMap<'a> {
        &self.host_functions
    }

    fn get_host_funcs_mut(&mut self) -> &mut HostFunctionsMap<'a> {
        &mut self.host_functions
    }
}

impl<'a> CallHostFunction<'a> for Sandbox<'a> {}

impl<'a> CallGuestFunction<'a> for Sandbox<'a> {}

impl<'a> RestoreSandbox for Sandbox<'a> {}

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

impl<'a> GuestMgr for Sandbox<'a> {
    fn get_executing_guest_call(&self) -> &AtomicI32 {
        &self.executing_guest_call
    }

    fn get_executing_guest_call_mut(&mut self) -> &mut AtomicI32 {
        &mut self.executing_guest_call
    }

    fn increase_num_runs(&mut self) {
        self.num_runs += 1;
    }

    fn get_num_runs(&self) -> i32 {
        self.num_runs
    }

    fn needs_state_reset(&self) -> bool {
        self.needs_state_reset
    }

    fn set_needs_state_reset(&mut self, val: bool) {
        self.needs_state_reset = val;
    }

    fn as_guest_mgr(&self) -> &dyn GuestMgr {
        self
    }

    fn as_guest_mgr_mut(&mut self) -> &mut dyn GuestMgr {
        self
    }
}

impl<'a> MemMgr for Sandbox<'a> {
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

    /// Check for a guest error and return an `Err` if one was found,
    /// and `Ok` if one was not found.
    /// TODO: remove this when we hook it up to the rest of the
    /// sandbox in https://github.com/deislabs/hyperlight/pull/727
    #[allow(unused)]
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

    #[allow(unused)]
    pub(crate) fn dispatch_call_from_host(
        &mut self,
        function_name: String,
        return_type: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<i32> {
        let p_dispatch = self.mem_mgr.get_pointer_to_dispatch_function()?;

        let fc = FunctionCall::new(function_name, args, FunctionCallType::Host, return_type);

        let buffer: Vec<u8> = fc.try_into()?;

        self.mem_mgr.write_guest_function_call(&buffer)?;

        #[allow(clippy::if_same_then_else)]
        if self.mem_mgr.is_in_process() {
            let dispatch: fn() = unsafe { std::mem::transmute(p_dispatch) };
            // Q: Why does this function not take `args` and doesn't return `return_type`?
            //
            // A: That's because we've already written the function call details to memory
            // with `self.mem_mgr.write_guest_function_call(&buffer)?;`
            // and the `dispatch` function can directly access that via shared memory.
            dispatch();
        } else {
            // TODO: For this, we're missing some sort of API
            // to get the current Hypervisor set by `set_up_hypervisor_partition`
            // in `UninitializedSandbox`. Once that's done, we should be able to
            // to something like this: `self.mem_mgr.get_hypervisor().dispatch(...)`
        }

        self.check_stack_guard()?;
        self.check_for_guest_error()?;

        self.mem_mgr.get_return_value()
    }
}
