use super::{guest_funcs::dispatch_call_from_host, leaked_outb::LeakedOutBWrapper};
use crate::HypervisorWrapperMgr;
use crate::MemMgrWrapperGetter;
use crate::Result;
use crate::{
    func::{guest::GuestFunction, ParameterValue, ReturnType, ReturnValue},
    sandbox_state::sandbox::Sandbox,
    HypervisorWrapper, MemMgrWrapper, UninitializedSandbox,
};
use std::marker::PhantomData;

/// A sandbox implementation that supports calling no more than 1 guest
/// function
pub struct SingleUseSandbox<'a> {
    pub(super) mem_mgr: MemMgrWrapper,
    pub(super) hv: HypervisorWrapper<'a>,
    /// Adding this field to ensure `SingleUseSandbox` is not `Send` and
    /// instances thereof cannot be sent to a different thread
    ///
    /// See https://github.com/rust-lang/rust/issues/68318#issuecomment-1066221968
    /// for more detail
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

    /// Call the guest function called `func_name` with the given arguments
    /// `args`, and expect the return value have the same type as
    /// `func_ret_type`.
    pub fn call_guest_function_by_name(
        mut self,
        name: &str,
        ret: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<ReturnValue> {
        dispatch_call_from_host(&mut self, name, ret, args)
    }

    /// Execute the given callback function `func` in the context of a guest
    /// calling "session".
    ///
    /// The `func` parameter you pass will be called after `self` is prepared
    /// to make 1 or more guest calls. Then, `func` will be called, and given
    /// a `MultiUseSandbox` it can use to execute the needed guest calls.
    /// After `func` returns, `self`'s state will be cleaned up, indicating
    /// the execution is complete.
    ///
    /// Importantly, this function's first parameter is `self`, which means
    /// it "consumes" `self` when you call it, and the rust compiler will
    /// not allow you to use it thereafter.
    pub fn execute_in_host<Fn: GuestFunction<SingleUseSandbox<'a>, Ret>, Ret>(
        self,
        func: Fn,
    ) -> Result<Ret> {
        func.call(self)
    }
}

impl<'a> Sandbox for SingleUseSandbox<'a> {
    fn is_reusable(&self) -> bool {
        false
    }

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

impl<'a> HypervisorWrapperMgr<'a> for SingleUseSandbox<'a> {
    fn get_hypervisor_wrapper(&self) -> &HypervisorWrapper<'a> {
        &self.hv
    }
    fn get_hypervisor_wrapper_mut(&mut self) -> &mut HypervisorWrapper<'a> {
        &mut self.hv
    }
}

impl<'a> MemMgrWrapperGetter for SingleUseSandbox<'a> {
    fn get_mem_mgr_wrapper(&self) -> &MemMgrWrapper {
        &self.mem_mgr
    }
    fn get_mem_mgr_wrapper_mut(&mut self) -> &mut MemMgrWrapper {
        &mut self.mem_mgr
    }
}
