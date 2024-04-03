use super::{leaked_outb::LeakedOutBWrapper, WrapperGetter};
use super::{HypervisorWrapper, MemMgrWrapper, UninitializedSandbox};
use crate::func::call_ctx::SingleUseGuestCallContext;
use crate::sandbox_state::sandbox::Sandbox;
use crate::Result;
use hyperlight_flatbuffers::flatbuffer_wrappers::function_types::{
    ParameterValue, ReturnType, ReturnValue,
};
use std::marker::PhantomData;
use tracing::{instrument, Span};

/// A sandbox implementation that supports calling no more than 1 guest
/// function
pub struct SingleUseSandbox<'a> {
    pub(super) mem_mgr: MemMgrWrapper,
    pub(super) hv: HypervisorWrapper<'a>,
    /// This field is a "marker type" to ensure `SingleUseSandbox` is not
    /// `Send` and thus, instances thereof cannot be sent to a different
    /// thread. This feature is important because the owner of a single-use
    /// sandbox should be the only owner, and not be able to move or share
    /// it across threads.
    ///
    /// See https://github.com/rust-lang/rust/issues/68318#issuecomment-1066221968
    /// for more detail on marker types.
    make_unsend: PhantomData<*mut ()>,
    /// This field is a representation of a leaked outb handler. It exists
    /// only to support in-process mode, and will be set to None in all
    /// other cases. It is never actually used, and is only here so it
    /// will be dropped and the leaked memory is cleaned up.
    ///
    /// See documentation for `LeakedOutB` for more details
    _leaked_outb: Option<LeakedOutBWrapper<'a>>,
}

impl<'a> SingleUseSandbox<'a> {
    /// Move an `UninitializedSandbox` into a new `SingleUseSandbox` instance.
    ///
    /// This function is not equivalent to doing an `evolve` from uninitialized
    /// to initialized. It only copies values from `val` to the new returned
    /// `SingleUseSandbox` instance, and does not execute any intialization
    /// logic on the guest. We want to ensure that, when users request to
    /// convert an `UninitializedSandbox` to a `SingleUseSandbox`,
    /// initialization logic is always run, so we are purposely making this
    /// function not publicly exposed. Finally, although it looks like it should be
    /// in a `From` implementation, it is purposely not, because external
    /// users would then see it and be able to use it.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn from_uninit(
        val: UninitializedSandbox<'a>,
        leaked_outb: Option<LeakedOutBWrapper<'a>>,
    ) -> SingleUseSandbox<'a> {
        Self {
            mem_mgr: val.mgr,
            hv: val.hv,
            make_unsend: PhantomData,
            _leaked_outb: leaked_outb,
        }
    }

    /// Create a new `SingleUseCallContext` suitable for making 0 or more
    /// calls to guest functions within the same context.
    ///
    /// Since this function consumes `self`, the returned
    /// `SingleUseGuestCallContext` is guaranteed mutual exclusion for calling
    /// functions within the sandbox. This guarantee is enforced at compile
    /// time, and no locks, atomics, or any other mutual exclusion mechanisms
    /// are used at runtime.
    ///
    /// Since this is a `SingleUseSandbox`, the returned
    /// context cannot be converted back into the original `SingleUseSandbox`.
    /// When it's dropped, all the resources of the context and sandbox are
    /// released at once.
    ///
    /// Example usage (compiled as a "no_run" doctest since the test binary
    /// will not be found):
    ///
    /// ```no_run
    /// use hyperlight_host::sandbox::{UninitializedSandbox, SingleUseSandbox};
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
    /// let sbox: SingleUseSandbox = u_sbox.evolve(Noop::default()).unwrap();
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
    /// }
    /// // You can make further calls with the same context if you want.
    /// // Otherwise, `ctx` will be dropped and all resources, including the
    /// // underlying `SingleUseSandbox`, will be released and no further
    //  // contexts can be created from that sandbox.
    /// ```
    #[instrument(skip_all, parent = Span::current())]
    pub fn new_call_context(self) -> SingleUseGuestCallContext<'a> {
        SingleUseGuestCallContext::start(self)
    }

    /// Convenience for the following:
    ///
    /// `self.new_call_context().call(name, ret, args)`
    #[instrument(err(Debug), skip(self,args), parent = Span::current())]
    pub fn call_guest_function_by_name(
        self,
        name: &str,
        ret: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<ReturnValue> {
        self.new_call_context().call(name, ret, args)
    }
}

impl<'a> WrapperGetter<'a> for SingleUseSandbox<'a> {
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

impl<'a> Sandbox for SingleUseSandbox<'a> {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn is_reusable(&self) -> bool {
        false
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn check_stack_guard(&self) -> Result<bool> {
        self.mem_mgr.check_stack_guard()
    }
}

impl<'a> std::fmt::Debug for SingleUseSandbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SingleUseSandbox")
            .field("stack_guard", &self.mem_mgr.get_stack_cookie())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::sandbox::SandboxConfiguration;
    use crate::sandbox_state::sandbox::EvolvableSandbox;
    use crate::{sandbox_state::transition::Noop, GuestBinary};
    use crate::{SingleUseSandbox, UninitializedSandbox};
    use hyperlight_flatbuffers::flatbuffer_wrappers::function_types::{ParameterValue, ReturnType};
    use hyperlight_testing::simple_guest_as_string;

    // Tests to ensure that many (1000) function calls can be made in a call context with a small stack (1K) and heap(14K).
    // This test effectively ensures that the stack is being properly reset after each call and we are not leaking memory in the Guest.
    #[test]
    fn test_with_small_stack_and_heap() {
        let cfg = {
            let mut cfg = SandboxConfiguration::default();
            cfg.set_heap_size(14 * 1024);
            cfg.set_stack_size(14 * 1024);
            Some(cfg)
        };

        let sbox1: SingleUseSandbox = {
            let path = simple_guest_as_string().unwrap();
            let u_sbox =
                UninitializedSandbox::new(GuestBinary::FilePath(path), cfg, None, None).unwrap();
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

        let sbox2: SingleUseSandbox = {
            let path = simple_guest_as_string().unwrap();
            let u_sbox =
                UninitializedSandbox::new(GuestBinary::FilePath(path), cfg, None, None).unwrap();
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
}
