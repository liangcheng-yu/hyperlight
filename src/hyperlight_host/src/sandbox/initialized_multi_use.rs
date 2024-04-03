use super::{host_funcs::HostFuncsWrapper, leaked_outb::LeakedOutBWrapper, WrapperGetter};
use super::{HypervisorWrapper, MemMgrWrapper, UninitializedSandbox};
use crate::func::call_ctx::MultiUseGuestCallContext;
use crate::sandbox::snapshot::Snapshot;
use crate::sandbox_state::{
    sandbox::{DevolvableSandbox, Sandbox},
    transition::Noop,
};
use crate::Result;
use hyperlight_flatbuffers::flatbuffer_wrappers::function_types::{
    ParameterValue, ReturnType, ReturnValue,
};
use std::sync::{Arc, Mutex};
use tracing::{instrument, Span};

/// A sandbox that supports calling any number of guest functions, without
/// any limits to how many.
#[derive(Clone)]
pub struct MultiUseSandbox<'a> {
    pub(super) host_funcs: Arc<Mutex<HostFuncsWrapper<'a>>>,
    pub(crate) num_runs: i32,
    pub(super) mem_mgr: MemMgrWrapper,
    pub(super) run_from_process_memory: bool,
    pub(super) hv: HypervisorWrapper<'a>,
    /// See the documentation for `SingleUseSandbox::_leaked_out_b` for
    /// details on the purpose of this field.
    _leaked_outb: Arc<Option<LeakedOutBWrapper<'a>>>,
    pub(super) mem_snapshots: Arc<Mutex<Vec<Snapshot>>>,
}

impl<'a> MultiUseSandbox<'a> {
    /// Move an `UninitializedSandbox` into a new `MultiUseSandbox` instance.
    ///
    /// This function is not equivalent to doing an `evolve` from uninitialized
    /// to initialized, and is purposely not exposed publicly outside the crate
    /// (as a `From` implementation would be)
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn from_uninit(
        val: UninitializedSandbox<'a>,
        leaked_outb: Option<LeakedOutBWrapper<'a>>,
    ) -> MultiUseSandbox<'a> {
        Self {
            host_funcs: val.host_funcs,
            num_runs: 0,
            mem_mgr: val.mgr,
            run_from_process_memory: val.run_from_process_memory,
            hv: val.hv,
            _leaked_outb: Arc::new(leaked_outb),
            mem_snapshots: val.mem_snapshots.clone(),
        }
    }

    /// Create a new `MultiUseCallContext` suitable for making 0 or more
    /// calls to guest functions within the same context.
    ///
    /// Since this function consumes `self`, the returned
    /// `MultiUseGuestCallContext` is guaranteed mutual exclusion for calling
    /// functions within the sandbox. This guarantee is enforced at compile
    /// time, and no locks, atomics, or any other mutual exclusion mechanisms
    /// are used at rumtime.
    ///
    /// If you have called this function, have a `MultiUseGuestCallContext`,
    /// and wish to "return" it to a `MultiUseSandbox`, call the `finish`
    /// method on the context.
    ///
    /// /// Example usage (compiled as a "no_run" doctest since the test binary
    /// will not be found):
    ///
    /// ```no_run
    /// use hyperlight_host::sandbox::{UninitializedSandbox, MultiUseSandbox};
    /// use hyperlight_flatbuffers::flatbuffer_wrappers::function_types::{ReturnType, ParameterValue, ReturnValue};
    /// use hyperlight_host::sandbox_state::sandbox::EvolvableSandbox;
    /// use hyperlight_host::sandbox_state::transition::Noop;
    /// use hyperlight_host::GuestBinary;
    ///
    /// // First, create a new uninitialized sandbox, then evolve it to become
    /// // an initialized, single-use one.
    /// let u_sbox = UninitializedSandbox::new(
    ///     GuestBinary::FilePath("some_guest_binary".to_string()),
    ///     None,
    ///     None,
    ///     None,
    /// ).unwrap();
    /// let sbox: MultiUseSandbox = u_sbox.evolve(Noop::default()).unwrap();
    /// // Next, create a new call context from the single-use sandbox.
    /// // After this line, your code will not compile if you try to use the
    /// // original `sbox` variable.
    /// let mut ctx = sbox.new_call_context();
    ///
    /// // Do a guest call with the context. Assues that the loaded binary
    /// // ("some_guest_binary") has a function therein called "SomeGuestFunc"
    /// // that takes a single integer argument and returns an integer.
    /// match ctx.call(
    ///     "SomeGuestFunc",
    ///     ReturnType::Int,
    ///     Some(vec![ParameterValue::Int(1)])
    /// ) {
    ///     Ok(ReturnValue::Int(i)) => println!(
    ///         "got successful return value {}",
    ///         i,
    ///     ),
    ///     other => panic!(
    ///         "failed to get return value as expected ({:?})",
    ///         other,
    ///     ),
    /// };
    /// // You can make further calls with the same context if you want.
    /// // Otherwise, `ctx` will be dropped and all resources, including the
    /// // underlying `MultiUseSandbox`, will be released and no further
    /// // contexts can be created from that sandbox.
    /// //
    /// // If you want to avoid
    /// // that behavior, call `finish` to convert the context back to
    /// // the original `MultiUseSandbox`, as follows:
    /// let _orig_sbox = ctx.finish();
    /// // Now, you can operate on the original sandbox again (i.e. add more
    /// // host functions etc...), create new contexts, and so on.
    /// ```
    #[instrument(skip_all, parent = Span::current())]
    pub fn new_call_context(self) -> MultiUseGuestCallContext<'a> {
        MultiUseGuestCallContext::start(self)
    }

    /// Convenience method for the following:
    ///
    /// `self.new_call_context()?.call(func_name, func_ret_type, args)`
    #[instrument(err(Debug), skip(self, args), parent = Span::current())]
    pub fn call_guest_function_by_name(
        self,
        func_name: &str,
        func_ret_type: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<(Self, ReturnValue)> {
        let mut ctx = self.new_call_context();
        let res = ctx.call(func_name, func_ret_type, args)?;
        let sbx = ctx.finish()?;
        Ok((sbx, res))
    }

    /// Reset the Sandbox's state
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn reset_state(&mut self) -> Result<()> {
        self.restore_state()?;
        self.num_runs += 1;

        Ok(())
    }

    /// Restore the Sandbox's state
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn restore_state(&mut self) -> Result<()> {
        let mem_mgr = self.mem_mgr.get_mgr_mut();
        mem_mgr.restore_state()?;
        if !self.run_from_process_memory {
            let orig_rsp = self.hv.get_hypervisor()?.orig_rsp()?;
            self.hv.get_hypervisor()?.reset_rsp(orig_rsp)?;
        }

        Ok(())
    }
}

impl<'a> WrapperGetter<'a> for MultiUseSandbox<'a> {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_mgr(&self) -> &MemMgrWrapper {
        &self.mem_mgr
    }
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_mgr_mut(&mut self) -> &mut MemMgrWrapper {
        &mut self.mem_mgr
    }
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_hv(&self) -> &HypervisorWrapper<'a> {
        &self.hv
    }
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_hv_mut(&mut self) -> &mut HypervisorWrapper<'a> {
        &mut self.hv
    }
}

impl<'a> Sandbox for MultiUseSandbox<'a> {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn is_reusable(&self) -> bool {
        true
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
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
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn devolve(
        self,
        _tsn: Noop<MultiUseSandbox<'a>, UninitializedSandbox<'a>>,
    ) -> Result<UninitializedSandbox<'a>> {
        let run_from_proc = self.run_from_process_memory;
        let mut ret = UninitializedSandbox::from_multi_use(self);
        ret.mgr.as_mut().restore_state()?;
        if run_from_proc {
            let orig_rsp = ret.hv.get_hypervisor()?.orig_rsp()?;
            ret.hv.get_hypervisor()?.reset_rsp(orig_rsp)?;
        }
        Ok(ret)
    }
}
