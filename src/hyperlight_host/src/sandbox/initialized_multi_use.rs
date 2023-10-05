use super::host_funcs::HostFuncsWrapper;
use super::initialized_multi_use_release::{ShouldRelease, ShouldReset};
use super::{guest_call_exec::ExecutingGuestCall, guest_funcs::dispatch_call_from_host};
use crate::{
    func::{guest::GuestFunction, ParameterValue, ReturnType, ReturnValue},
    mem::ptr::{GuestPtr, RawPtr},
    sandbox_state::{
        sandbox::{DevolvableSandbox, Sandbox},
        transition::Noop,
    },
    GuestMgr, HypervisorWrapper, HypervisorWrapperMgr, MemMgrWrapper, MemMgrWrapperGetter,
    UninitializedSandbox,
};
use anyhow::{bail, Result};
use std::sync::{Arc, Mutex};
use tracing::instrument;

/// A sandbox that supports calling any number of guest functions, without
/// any limits to how many
#[derive(Clone)]
pub struct MultiUseSandbox<'a> {
    pub(super) host_funcs: Arc<Mutex<HostFuncsWrapper<'a>>>,
    needs_state_reset: bool,
    executing_guest_call: ExecutingGuestCall,
    num_runs: i32,
    pub(super) mem_mgr: MemMgrWrapper,
    pub(super) run_from_process_memory: bool,
    pub(super) hv: HypervisorWrapper<'a>,
}

impl<'a> MultiUseSandbox<'a> {
    /// Move an `UninitializedSandbox` into a new `MultiUseSandbox` instance.
    ///
    /// This function is not equivalent to doing an `evolve` from uninitialized
    /// to initialized, and is purposely not exposed publicly outside the crate
    /// (as a `From` implementation would be)
    pub(super) fn from_uninit(val: UninitializedSandbox<'a>) -> MultiUseSandbox<'a> {
        Self {
            host_funcs: val.host_funcs,
            needs_state_reset: false,
            executing_guest_call: ExecutingGuestCall::new(0),
            num_runs: 0,
            mem_mgr: val.mgr,
            run_from_process_memory: val.run_from_process_memory,
            hv: val.hv,
        }
    }

    /// Call the guest function called `func_name` with the given arguments
    /// `args`, and expect the return value have the same type as
    /// `func_ret_type`.
    #[instrument]
    pub fn call_guest_function_by_name(
        &mut self,
        func_name: &str,
        func_ret_type: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<ReturnValue> {
        let should_reset = self.enter_method();

        // We prefix the variable below w/ an underscore because it is
        // 'technically' unused, as our purpose w/ it is just for it to
        // go out of scope and call its' custom `Drop` `impl`.
        let mut _sr = ShouldReset::new(should_reset, self.clone());

        if should_reset {
            self.reset_state()?;
        }

        dispatch_call_from_host(self, func_name, func_ret_type, args)
    }

    /// Execute the given callback function `func` in the context of a guest
    /// function calling "session".
    ///
    /// The `func` parameter you pass will be called after `self` is prepared
    /// to make 1 or more guest calls. Then, `func` will be called, and given
    /// a `MultiUseSandbox` it can use to execute the needed guest calls.
    /// After `func` returns, `self`'s state will be cleaned up, indicating
    /// the execution is complete.
    pub fn execute_in_host<Fn: GuestFunction<Arc<Mutex<MultiUseSandbox<'a>>>, Ret>, Ret>(
        &mut self,
        func: Fn,
    ) -> Result<Ret> {
        let sbox_arc = Arc::new(Mutex::new(self.clone()));
        let mut sd = ShouldRelease::new(false, sbox_arc.clone());
        if sbox_arc
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
            .get_executing_guest_call_mut()
            .compare_exchange(0, 1)
            .map_err(|_| anyhow::anyhow!("Failed to verify status of guest function execution"))?
            != 0
        {
            bail!("Guest call already in progress");
        }

        sd.toggle();
        sbox_arc
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
            .reset_state()?;

        func.call(sbox_arc)
        // ^^^ ensures that only one call can be made concurrently
        // because `GuestFunction` is implemented for `Arc<Mutex<T>>`
        // so we'll be locking on the function call. There are tests
        // below that demonstrate this.
    }

    pub(super) fn set_needs_state_reset(&mut self, val: bool) {
        self.needs_state_reset = val;
    }

    pub(super) fn get_executing_guest_call_mut(&mut self) -> &mut ExecutingGuestCall {
        &mut self.executing_guest_call
    }

    /// Reset the Sandbox's state
    pub(super) fn reset_state(&mut self) -> Result<()> {
        self.restore_state()?;
        self.num_runs += 1;

        Ok(())
    }

    /// Reset the internal guest function run counter to 0.
    ///
    /// TODO: this is a hack to allow hyperlight-wasm to properly
    /// initialize its structures while also ensuring it can schedule
    /// subsequent guest calls properly
    pub fn reset_num_runs(&mut self) {
        self.num_runs = 0
    }

    /// Restore the Sandbox's state
    fn restore_state(&mut self) -> Result<()> {
        if self.needs_state_reset {
            let mem_mgr = self.mem_mgr.get_mgr_mut();
            mem_mgr.restore_state()?;
            if !self.run_from_process_memory {
                let orig_rsp = self.hv.orig_rsp()?;
                self.hv.reset_rsp(orig_rsp)?;
            }
            self.set_needs_state_reset(false);
        }

        Ok(())
    }
}

impl<'a> Sandbox for MultiUseSandbox<'a> {
    fn is_reusable(&self) -> bool {
        true
    }

    fn check_stack_guard(&self) -> Result<bool> {
        self.mem_mgr.check_stack_guard()
    }
}

impl<'a> std::fmt::Debug for MultiUseSandbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiUseSandbox")
            .field("stack_guard", &self.mem_mgr.get_stack_cookie())
            .finish()
    }
}

impl<'a> GuestMgr for MultiUseSandbox<'a> {
    fn get_executing_guest_call(&self) -> &ExecutingGuestCall {
        &self.executing_guest_call
    }

    fn get_executing_guest_call_mut(&mut self) -> &mut ExecutingGuestCall {
        &mut self.executing_guest_call
    }

    fn increase_num_runs(&mut self) {
        self.num_runs += 1
    }

    fn get_num_runs(&self) -> i32 {
        self.num_runs
    }

    /// Checks if the `Sandbox` needs state resetting.
    fn needs_state_reset(&self) -> bool {
        self.needs_state_reset
    }

    fn set_needs_state_reset(&mut self, val: bool) {
        self.needs_state_reset = val;
    }

    /// Get immutable reference as `Box<dyn GuestMgr>`
    fn as_guest_mgr(&self) -> &dyn GuestMgr {
        self
    }

    fn as_guest_mgr_mut(&mut self) -> &mut dyn GuestMgr {
        self
    }
}

impl<'a> HypervisorWrapperMgr<'a> for MultiUseSandbox<'a> {
    fn get_hypervisor_wrapper(&self) -> &HypervisorWrapper<'a> {
        &self.hv
    }
    fn get_hypervisor_wrapper_mut(&mut self) -> &mut HypervisorWrapper<'a> {
        &mut self.hv
    }
}

impl<'a> MemMgrWrapperGetter for MultiUseSandbox<'a> {
    fn get_mem_mgr_wrapper(&self) -> &MemMgrWrapper {
        &self.mem_mgr
    }
    fn get_mem_mgr_wrapper_mut(&mut self) -> &mut MemMgrWrapper {
        &mut self.mem_mgr
    }
}

impl<'a>
    DevolvableSandbox<
        MultiUseSandbox<'a>,
        UninitializedSandbox<'a>,
        Noop<MultiUseSandbox<'a>, UninitializedSandbox<'a>>,
    > for MultiUseSandbox<'a>
{
    /// Consume `self` and move it back to an `UninitializedSandbox`. The
    /// devolving process entails the following:
    ///
    /// - If `self` was a recyclable sandbox, restore its state from a
    /// previous state snapshot
    /// - If `self` was using in-process mode, reset the stack pointer
    /// (RSP register, to be specific) to what it was when the sandbox
    /// was first created.
    fn devolve(
        self,
        _tsn: Noop<MultiUseSandbox<'a>, UninitializedSandbox<'a>>,
    ) -> Result<UninitializedSandbox<'a>> {
        let run_from_proc = self.run_from_process_memory;
        let mut ret = UninitializedSandbox::from_multi_use(self);
        ret.mgr.as_mut().restore_state()?;
        if run_from_proc {
            let orig_rsp_raw = ret.hv.get_hypervisor()?.orig_rsp()?;
            let orig_rsp = GuestPtr::try_from(RawPtr::from(orig_rsp_raw))?;
            ret.hv.reset_rsp(orig_rsp)?;
        }
        Ok(ret)
    }
}

/// This `Drop` implementation is applicable to in-process execution only,
/// and thus is applicable to windows builds only.
///
/// See the `super::initialized::drop_impl` method for more detail on why
/// this exists and how it works.
#[cfg(target_os = "windows")]
impl<'a> Drop for MultiUseSandbox<'a> {
    fn drop(&mut self) {
        super::initialized::drop_impl(self.mem_mgr.as_ref())
    }
}
