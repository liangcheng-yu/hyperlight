use super::{host_funcs::HostFuncsWrapper, leaked_outb::LeakedOutBWrapper, WrapperGetter};
use super::{HypervisorWrapper, MemMgrWrapper, UninitializedSandbox};
use crate::func::call_ctx::MultiUseGuestCallContext;
use crate::func::guest_dispatch::call_function_on_guest;
use crate::hypervisor::hypervisor_handler::kill_hypervisor_handler_thread;
use crate::sandbox_state::sandbox::EvolvableSandbox;
use crate::sandbox_state::transition::MultiUseContextCallback;
use crate::sandbox_state::{
    sandbox::{DevolvableSandbox, Sandbox},
    transition::Noop,
};
use crate::Result;
use hyperlight_common::flatbuffer_wrappers::function_types::{
    ParameterValue, ReturnType, ReturnValue,
};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tracing::{instrument, Span};

/// A sandbox that supports being used Multiple times.
/// The implication of being used multiple times is two-fold:
///
/// 1. The sandbox can be used to call guest functions multiple times, each time a
///  guest function is called the state of the sandbox is reset to the state it was in before the call was made.
///
/// 2. A MultiUseGuestCallContext can be created from the sandbox and used to make multiple guest function calls to the Sandbox.
///  in this case the state of the sandbox is not reset until the context is finished and the `MultiUseSandbox` is returned.
pub struct MultiUseSandbox<'a> {
    pub(super) host_funcs: Arc<Mutex<HostFuncsWrapper>>,
    pub(crate) mem_mgr: MemMgrWrapper,
    pub(super) run_from_process_memory: bool,
    pub(super) hv: HypervisorWrapper,
    pub(super) join_handle: Option<JoinHandle<Result<()>>>,
    /// See the documentation for `SingleUseSandbox::_leaked_out_b` for
    /// details on the purpose of this field.
    _leaked_outb: Arc<Option<LeakedOutBWrapper<'a>>>,
}

// We need to implement drop to join the
// threads, because, otherwise, we will
// be leaking a thread with every
// sandbox that is dropped. This was initially
// caught by our benchmarks that created a ton of
// sandboxes and caused the system to run out of
// resources. Now, this is covered by the test:
// `create_1000_sandboxes`.
impl Drop for MultiUseSandbox<'_> {
    fn drop(&mut self) {
        if self.join_handle.is_some() {
            match kill_hypervisor_handler_thread(self) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("[LEAKED THREAD] Failed to kill hypervisor handler thread when dropping MultiUseSandbox: {:?}", e);
                }
            }
        } else {
            log::debug!("[LEAKED THREAD] Running from C API configured Sandbox, no Hypervisor Handler thread to kill.");
        }
    }
}

impl<'a> MultiUseSandbox<'a> {
    /// Move an `UninitializedSandbox` into a new `MultiUseSandbox` instance.
    ///
    /// This function is not equivalent to doing an `evolve` from uninitialized
    /// to initialized, and is purposely not exposed publicly outside the crate
    /// (as a `From` implementation would be)
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    pub(super) fn from_uninit(
        val: UninitializedSandbox,
        join_handle: Option<JoinHandle<Result<()>>>,
        leaked_outb: Option<LeakedOutBWrapper<'a>>,
    ) -> MultiUseSandbox<'a> {
        Self {
            host_funcs: val.host_funcs,
            mem_mgr: val.mgr,
            run_from_process_memory: val.run_from_process_memory,
            hv: val.hv,
            join_handle,
            _leaked_outb: Arc::new(leaked_outb),
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
    /// Example usage (compiled as a "no_run" doctest since the test binary
    /// will not be found):
    ///
    /// ```no_run
    /// use hyperlight_host::sandbox::{UninitializedSandbox, MultiUseSandbox};
    /// use hyperlight_common::flatbuffer_wrappers::function_types::{ReturnType, ParameterValue, ReturnValue};
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

    /// Call a guest function by name, with the given return type and arguments.
    #[instrument(err(Debug), skip(self, args), parent = Span::current())]
    pub fn call_guest_function_by_name(
        &mut self,
        func_name: &str,
        func_ret_type: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<ReturnValue> {
        let res = call_function_on_guest(self, func_name, func_ret_type, args)?;
        self.restore_state()?;
        Ok(res)
    }

    /// Restore the Sandbox's state
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub(crate) fn restore_state(&mut self) -> Result<()> {
        let mem_mgr = self.mem_mgr.unwrap_mgr_mut();
        mem_mgr.restore_state_from_last_snapshot()?;
        if !self.run_from_process_memory {
            let orig_rsp = self.hv.get_hypervisor_lock()?.orig_rsp()?;
            self.hv.get_hypervisor_lock()?.reset_rsp(orig_rsp)?;
        }

        Ok(())
    }
}

impl<'a> WrapperGetter for MultiUseSandbox<'a> {
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_mgr_wrapper(&self) -> &MemMgrWrapper {
        &self.mem_mgr
    }
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_mgr_wrapper_mut(&mut self) -> &mut MemMgrWrapper {
        &mut self.mem_mgr
    }
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_hv(&self) -> &HypervisorWrapper {
        &self.hv
    }
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_hv_mut(&mut self) -> &mut HypervisorWrapper {
        &mut self.hv
    }
}

impl<'a> Sandbox for MultiUseSandbox<'a> {
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn check_stack_guard(&self) -> Result<bool> {
        self.mem_mgr.check_stack_guard()
    }

    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_hypervisor_wrapper_mut(&mut self) -> &mut HypervisorWrapper {
        &mut self.hv
    }

    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_hypervisor_handler_thread_mut(&mut self) -> &mut Option<JoinHandle<Result<()>>> {
        &mut self.join_handle
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
        UninitializedSandbox,
        Noop<MultiUseSandbox<'a>, UninitializedSandbox>,
    > for MultiUseSandbox<'a>
{
    /// Consume `self` and move it back to an `UninitializedSandbox`. The
    /// devolving process entails the following:
    ///
    /// - If `self` was a recyclable sandbox, restore its state from a
    /// previous state snapshot
    ///
    /// TODO: Why are we doing the reset RSP? Its seems wrong to me , we are not using hypervisor in in process mode, why doesnt this fail? Do we have test for it?
    /// - If `self` was using in-process mode, reset the stack pointer
    /// (RSP register, to be specific) to what it was when the sandbox
    /// was first created.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn devolve(
        self,
        _tsn: Noop<MultiUseSandbox<'a>, UninitializedSandbox>,
    ) -> Result<UninitializedSandbox> {
        let run_from_proc = self.run_from_process_memory;
        let mut ret = UninitializedSandbox::from_multi_use(self);
        ret.mgr.as_mut().pop_and_restore_state_from_snapshot()?;
        if run_from_proc {
            let orig_rsp = ret.hv.get_hypervisor_lock()?.orig_rsp()?;
            ret.hv.get_hypervisor_lock()?.reset_rsp(orig_rsp)?;
        }
        Ok(ret)
    }
}

impl<'a>
    DevolvableSandbox<
        MultiUseSandbox<'a>,
        MultiUseSandbox<'a>,
        Noop<MultiUseSandbox<'a>, MultiUseSandbox<'a>>,
    > for MultiUseSandbox<'a>
{
    /// Consume `self` and move it back to a `MultiUseSandbox` with previous state.
    ///
    /// The purpose of this function is to allow multiple states to be associated with a single MultiUseSandbox.
    ///
    /// An implementation such as HyperlightJs or HyperlightWasm can use this to call guest functions to load JS or WASM code and then evolve the sandbox causing state to be captured.
    /// The new MultiUseSandbox can then be used to call guest functions to execute the loaded code.
    /// The devolve can be used to return the MultiUseSandbox to the state before the code was loaded. Thus avoiding initialisation overhead
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn devolve(
        mut self,
        _tsn: Noop<MultiUseSandbox<'a>, MultiUseSandbox<'a>>,
    ) -> Result<MultiUseSandbox<'a>> {
        self.mem_mgr
            .unwrap_mgr_mut()
            .pop_and_restore_state_from_snapshot()?;
        Ok(self)
    }
}

impl<'a, F>
    EvolvableSandbox<
        MultiUseSandbox<'a>,
        MultiUseSandbox<'a>,
        MultiUseContextCallback<'a, MultiUseSandbox<'a>, F>,
    > for MultiUseSandbox<'a>
where
    F: FnOnce(&mut MultiUseGuestCallContext) -> Result<()> + 'a,
{
    /// The purpose of this function is to allow multiple states to be associated with a single MultiUseSandbox.
    ///
    /// An implementation such as HyperlightJs or HyperlightWasm can use this to call guest functions to load JS or WASM code and then evolve the sandbox causing state to be captured.
    /// The new MultiUseSandbox can then be used to call guest functions to execute the loaded code.
    ///
    /// The evolve function creates a new MutliUseCallContext which is then passed to a callback function  allowing the
    /// callback function to call guest functions as part of the evolve process, once the callback function  is complete
    /// the context is finished using a crate internal method that does not restore the prior state of the Sanbbox.
    /// It then creates a mew  memory snapshot on the snapshot stack and returns the MultiUseSandbox
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn evolve(
        self,
        transition_func: MultiUseContextCallback<'a, MultiUseSandbox<'a>, F>,
    ) -> Result<MultiUseSandbox<'a>> {
        let mut ctx = self.new_call_context();
        transition_func.call(&mut ctx)?;
        let mut sbox = ctx.finish_no_reset();
        sbox.mem_mgr.unwrap_mgr_mut().push_state()?;
        Ok(sbox)
    }
}

#[cfg(test)]
mod tests {
    use crate::func::call_ctx::MultiUseGuestCallContext;
    use crate::sandbox::SandboxConfiguration;
    use crate::sandbox_state::sandbox::DevolvableSandbox;
    use crate::sandbox_state::sandbox::EvolvableSandbox;
    use crate::sandbox_state::transition::MultiUseContextCallback;
    use crate::{sandbox_state::transition::Noop, GuestBinary};
    use crate::{MultiUseSandbox, UninitializedSandbox};
    use hyperlight_common::flatbuffer_wrappers::function_types::{
        ParameterValue, ReturnType, ReturnValue,
    };
    use hyperlight_testing::simple_guest_as_string;

    // Tests to ensure that many (1000) function calls can be made in a call context with a small stack (1K) and heap(14K).
    // This test effectively ensures that the stack is being properly reset after each call and we are not leaking memory in the Guest.
    #[test]
    fn test_with_small_stack_and_heap() {
        let mut cfg = SandboxConfiguration::default();
        cfg.set_heap_size(16 * 1024);
        cfg.set_stack_size(16 * 1024);

        let sbox1: MultiUseSandbox = {
            let path = simple_guest_as_string().unwrap();
            let u_sbox =
                UninitializedSandbox::new(GuestBinary::FilePath(path), Some(cfg), None, None)
                    .unwrap();
            u_sbox.evolve(Noop::default())
        }
        .unwrap();

        let mut ctx = sbox1.new_call_context();

        for _ in 0..1000 {
            ctx.call(
                "StackAllocate",
                ReturnType::Int,
                Some(vec![ParameterValue::Int(1)]),
            )
            .unwrap();
        }

        let sbox2: MultiUseSandbox = {
            let path = simple_guest_as_string().unwrap();
            let u_sbox =
                UninitializedSandbox::new(GuestBinary::FilePath(path), Some(cfg), None, None)
                    .unwrap();
            u_sbox.evolve(Noop::default())
        }
        .unwrap();

        let mut ctx = sbox2.new_call_context();

        for i in 0..1000 {
            ctx.call(
                "PrintUsingPrintf",
                ReturnType::Int,
                Some(vec![ParameterValue::String(
                    format!("Hello World {}\n", i).to_string(),
                )]),
            )
            .unwrap();
        }
    }

    /// Tests that evolving from MultiUseSandbox to MultiUseSandbox creates a new state
    /// and devolving from MultiUseSandbox to MultiUseSandbox restores the previous state
    #[test]
    fn evolve_devolve_handles_state_correctly() {
        let sbox1: MultiUseSandbox = {
            let path = simple_guest_as_string().unwrap();
            let u_sbox =
                UninitializedSandbox::new(GuestBinary::FilePath(path), None, None, None).unwrap();
            u_sbox.evolve(Noop::default())
        }
        .unwrap();

        let func = Box::new(|call_ctx: &mut MultiUseGuestCallContext| {
            call_ctx.call(
                "AddToStatic",
                ReturnType::Int,
                Some(vec![ParameterValue::Int(5)]),
            )?;
            Ok(())
        });
        let transition_func = MultiUseContextCallback::from(func);
        let mut sbox2 = sbox1.evolve(transition_func).unwrap();
        let res = sbox2
            .call_guest_function_by_name("GetStatic", ReturnType::Int, None)
            .unwrap();
        assert_eq!(res, ReturnValue::Int(5));
        let mut sbox3: MultiUseSandbox = sbox2.devolve(Noop::default()).unwrap();
        let res = sbox3
            .call_guest_function_by_name("GetStatic", ReturnType::Int, None)
            .unwrap();
        assert_eq!(res, ReturnValue::Int(0));
    }
}
