use super::guest_funcs::CallGuestFunction;
use super::hypervisor::HypervisorWrapperMgr;
use super::uninitialized::UninitializedSandbox;
use super::{guest_mgr::GuestMgr, host_funcs::HostFuncsWrapper};
use super::{
    hypervisor::HypervisorWrapper,
    mem_mgr::{MemMgrWrapper, MemMgrWrapperGetter},
};
use crate::flatbuffers::hyperlight::generated::ErrorCode;
#[cfg(target_os = "windows")]
use crate::hypervisor::handlers::OutBHandlerCaller;
use crate::sandbox_state::reset::RestoreSandbox;
use anyhow::{bail, Result};
use log::error;
use std::sync::atomic::AtomicI32;
use std::sync::{Arc, Mutex};

/// A container to atomically keep track of whether a sandbox is currently executing a guest call. Primarily
/// used to prevent concurrent execution of guest calls.
///
/// 0 = not executing a guest call
/// 1 = executing `execute_in_host`
/// 2 = executing a `call_guest_function_by_name`
#[derive(Clone)]
pub struct ExecutingGuestCall(Arc<AtomicI32>);

impl ExecutingGuestCall {
    /// Create a new `ExecutingGuestCall` with the provided value.
    pub fn new(val: i32) -> Self {
        Self(Arc::new(AtomicI32::new(val)))
    }

    /// Load the value of the `ExecutingGuestCall`.
    pub fn load(&self) -> i32 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Store a value in the `ExecutingGuestCall`.
    pub fn store(&self, val: i32) {
        self.0.store(val, std::sync::atomic::Ordering::SeqCst);
    }

    /// Compare and exchange the value of the `ExecutingGuestCall`.
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
pub struct Sandbox<'a> {
    // Registered host functions
    pub(crate) host_functions: Arc<Mutex<HostFuncsWrapper<'a>>>,
    // The memory manager for the sandbox.
    mgr: MemMgrWrapper,
    executing_guest_call: ExecutingGuestCall,
    needs_state_reset: bool,
    /// The number of times that this Sandbox has been run
    pub num_runs: i32,
    hv: HypervisorWrapper<'a>,
}

// If we are running in proc then we need to drop the outbhandlerwrapper that was leaked and written
// to shared memory
#[cfg(target_os = "windows")]
impl<'a> Drop for Sandbox<'a> {
    fn drop(&mut self) {
        let mgr = self.mgr.as_ref();
        let run_from_proc_mem = mgr.run_from_process_memory;
        if run_from_proc_mem {
            let mgr = self.mgr.as_ref();
            if let Ok(ctx) = mgr.get_outb_context() {
                if ctx != 0 {
                    let _outb_handlercaller: Box<Arc<Mutex<dyn OutBHandlerCaller>>> =
                        unsafe { Box::from_raw(ctx as *mut Arc<Mutex<dyn OutBHandlerCaller>>) };
                }
            }
        }
    }
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
            host_functions: val.host_funcs,
            mgr: val.mgr.clone(),
            executing_guest_call: ExecutingGuestCall::new(0),
            needs_state_reset: false,
            num_runs: 0,
            hv: val.hv,
        }
    }
}

impl<'a> MemMgrWrapperGetter for Sandbox<'a> {
    fn get_mem_mgr_wrapper(&self) -> &MemMgrWrapper {
        &self.mgr
    }

    fn get_mem_mgr_wrapper_mut(&mut self) -> &mut MemMgrWrapper {
        &mut self.mgr
    }
}

impl<'a> HypervisorWrapperMgr<'a> for Sandbox<'a> {
    fn get_hypervisor_wrapper(&self) -> &HypervisorWrapper<'a> {
        &self.hv
    }

    fn get_hypervisor_wrapper_mut(&mut self) -> &mut HypervisorWrapper<'a> {
        &mut self.hv
    }
}

impl<'a> CallGuestFunction<'a> for Sandbox<'a> {}

impl<'a> RestoreSandbox<'a> for Sandbox<'a> {}

impl<'a> crate::sandbox_state::sandbox::Sandbox for Sandbox<'a> {}

impl<'a> std::fmt::Debug for Sandbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sandbox")
            .field("stack_guard", &self.mgr.get_stack_cookie())
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

impl<'a> Sandbox<'a> {
    /// Check for a guest error and return an `Err` if one was found,
    /// and `Ok` if one was not found.
    /// TODO: remove this when we hook it up to the rest of the
    /// sandbox in https://github.com/deislabs/hyperlight/pull/727
    pub fn check_for_guest_error(&self) -> Result<()> {
        let guest_err = self.mgr.as_ref().get_guest_error()?;
        match guest_err.code {
            ErrorCode::NoError => Ok(()),
            ErrorCode::OutbError => match self.mgr.as_ref().get_host_error()? {
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
