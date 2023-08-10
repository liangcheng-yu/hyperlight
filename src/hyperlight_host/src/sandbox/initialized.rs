use super::guest_funcs::{CallGuestFunction, GuestFuncs};
use super::guest_mgr::GuestMgr;
use super::uninitialized::UninitializedSandbox;
use super::FunctionsMap;
use super::{host_funcs::CallHostFunction, mem_mgr::MemMgr, hypervisor::HypervisorWrapper};
use super::{host_funcs::CallHostPrint, outb::OutBAction};
use super::{host_funcs::HostFuncs, outb::outb_log};
use crate::flatbuffers::hyperlight::generated::ErrorCode;
use crate::func::types::ParameterValue;
use crate::mem::mgr::SandboxMemoryManager;
use crate::mem::mgr::STACK_COOKIE_LEN;
use crate::sandbox_state::reset::RestoreSandbox;
use anyhow::{bail, Result};
use log::error;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;

// 0 = not executing a guest call
// 1 = executing a guest call
// 2 = executing a dynamic guest call
#[derive(Clone)]
pub struct ExecutingGuestCall(Arc<AtomicI32>);

impl ExecutingGuestCall {
    pub fn new(val: i32) -> Self {
        Self(Arc::new(AtomicI32::new(val)))
    }

    pub fn load(&self) -> i32 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn store(&self, val: i32) {
        self.0.store(val, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn compare_exchange(&self, current: i32, new: i32) -> Result<i32> {
        self.0
            .compare_exchange(
                current,
                new,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .map_err(|_| anyhow::anyhow!("compare_exchange failed"))
    }
}

impl PartialEq for ExecutingGuestCall {
    fn eq(&self, other: &Self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
            == other.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Eq for ExecutingGuestCall {}

/// The primary mechanism to interact with VM partitions that run Hyperlight
/// guest binaries.
///
/// These can't be created directly. You must first create an
/// `UninitializedSandbox`, and then call `evolve` or `initialize` on it to
/// generate one of these.
#[allow(unused)]
#[derive(Clone, PartialEq, Eq)]
pub struct Sandbox<'a> {
    // Registered host functions
    host_functions: FunctionsMap<'a>,
    // The memory manager for the sandbox.
    mem_mgr: SandboxMemoryManager,
    stack_guard: [u8; STACK_COOKIE_LEN],
    executing_guest_call: ExecutingGuestCall,
    needs_state_reset: bool,
    num_runs: i32,
    dynamic_methods: FunctionsMap<'a>,
    hv: HypervisorWrapper,
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
            executing_guest_call: ExecutingGuestCall::new(0),
            needs_state_reset: false,
            num_runs: 0,
            dynamic_methods: val.get_dynamic_methods().clone(),
            hv: val.hv,
        }
    }
}

impl<'a> HostFuncs<'a> for Sandbox<'a> {
    fn get_host_funcs(&self) -> &FunctionsMap<'a> {
        &self.host_functions
    }

    fn get_host_funcs_mut(&mut self) -> &mut FunctionsMap<'a> {
        &mut self.host_functions
    }
}

impl<'a> GuestFuncs<'a> for Sandbox<'a> {
    fn get_dynamic_methods(&self) -> &FunctionsMap<'a> {
        &self.dynamic_methods
    }

    fn get_dynamic_methods_mut(&mut self) -> &mut FunctionsMap<'a> {
        &mut self.dynamic_methods
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
    fn get_executing_guest_call(&self) -> &ExecutingGuestCall {
        &self.executing_guest_call
    }

    fn get_executing_guest_call_mut(&mut self) -> &mut ExecutingGuestCall {
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
    pub fn check_for_guest_error(&self) -> Result<()> {
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
}
