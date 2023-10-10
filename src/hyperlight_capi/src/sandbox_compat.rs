extern crate hyperlight_host;
use super::{context::Context, handle::Handle, hdl::Hdl};
use hyperlight_host::log_then_return;
use hyperlight_host::new_error;
use hyperlight_host::sandbox_state::sandbox::Sandbox as GenericSandbox;
use hyperlight_host::Result;
use hyperlight_host::UninitializedSandbox;

/// Either an initialized or uninitialized sandbox. This enum is used
/// to allow our `Sandbox` wrapper type to store both an uninitailized
/// or initialized sandbox at the same time.
pub(crate) enum SandboxImpls {
    Uninit(Box<hyperlight_host::sandbox::uninitialized::UninitializedSandbox<'static>>),
    InitMultiUse(Box<hyperlight_host::MultiUseSandbox<'static>>),
    InitSingleUse(Box<hyperlight_host::SingleUseSandbox<'static>>),
}

/// This is the C API for the `Sandbox` type.
pub(crate) struct Sandbox {
    /// whether or not the sandbox stored herein, when initialized, should
    /// be a `MultiUseSandbox` or a `SingleUseSandbox`.
    should_recycle: bool,
    inner: SandboxImpls,
}

impl Sandbox {
    pub(super) fn from_uninit(
        should_recycle: bool,
        u_sbox: hyperlight_host::sandbox::uninitialized::UninitializedSandbox<'static>,
    ) -> Self {
        Self {
            should_recycle,
            inner: SandboxImpls::Uninit(Box::new(u_sbox)),
        }
    }

    /// Get an immutable reference to a `Sandbox` stored in `ctx` and
    /// pointed to by `handle`.
    pub(crate) fn get(ctx: &Context, hdl: Handle) -> Result<&Self> {
        Context::get(hdl, &ctx.sandboxes, |s| matches!(s, Hdl::Sandbox(_)))
    }

    /// Get a mutable reference to a `Sandbox` stored in `ctx` and
    /// pointed to by `handle`.
    pub(crate) fn get_mut(ctx: &mut Context, hdl: Handle) -> Result<&mut Self> {
        Context::get_mut(hdl, &mut ctx.sandboxes, |h| matches!(h, Hdl::Sandbox(_)))
    }

    /// Find the `Sandbox` in `ctx` referenced by `hdl`, remove it from `ctx`,
    /// then evolve it by calling `cb_fn`. Store `cb_fn`'s newly-returned
    /// `SandboxImpls` instance in `ctx`. On a successful return, the given
    /// `hdl` will point to the newly evolved sandbox.
    ///
    /// Returns an error in the following cases:
    ///
    /// - No `Sandbox` exists in `ctx` for the given handle
    /// - The `Sandbox` was found but it was already initialized
    /// - `cb_fn` returned an error
    ///
    /// On any error, the sandbox will be removed from `ctx`
    pub(super) fn evolve<CbFn>(ctx: &mut Context, hdl: Handle, cb_fn: CbFn) -> Result<()>
    where
        CbFn: FnOnce(bool, Box<UninitializedSandbox<'static>>) -> Result<SandboxImpls>,
    {
        let mut sbox = ctx
            .sandboxes
            .remove(&hdl.key())
            .ok_or(new_error!("no sandbox exists for the given handle"))?;
        let recycle = sbox.should_recycle;
        let new_sbox = match sbox.inner {
            SandboxImpls::Uninit(u_sbox) => cb_fn(recycle, u_sbox),
            _ => {
                log_then_return!("evolve: sandbox was already initialized");
            }
        }?;
        sbox.inner = new_sbox;
        ctx.sandboxes.insert(hdl.key(), sbox);
        Ok(())
    }

    /// Consume `self`, store it inside `ctx`, then return a new `Handle`
    /// referencing it
    pub(crate) fn register(self, ctx: &mut Context) -> Handle {
        Context::register(self, &mut ctx.sandboxes, Hdl::Sandbox)
    }

    /// Consume `self`, check if it holds a `sandbox::UninitializedSandbox`,
    /// and return an immutable reference to it if so.
    /// Otherwise, return an `Err`
    pub(crate) fn to_uninit(&self) -> Result<&UninitializedSandbox<'static>> {
        match &self.inner {
            SandboxImpls::Uninit(sbox) => Ok(sbox),
            _ => {
                log_then_return!(
                    "attempted to get immutable uninitialzied sandbox from an initialized one"
                );
            }
        }
    }
    /// Consume `self`, check if it holds a `sandbox::UninitializedSandbox`,
    /// and return an immutable reference to it if so.
    /// Otherwise, return an `Err`
    pub(crate) fn to_uninit_mut(&mut self) -> Result<&mut UninitializedSandbox<'static>> {
        match &mut self.inner {
            SandboxImpls::Uninit(sbox) => Ok(sbox),
            _ => {
                log_then_return!(
                    "attempted to get mutable uninitialzied sandbox from an initialized one"
                );
            }
        }
    }

    pub(crate) fn check_stack_guard(&self) -> Result<bool> {
        match &self.inner {
            SandboxImpls::Uninit(sbox) => sbox.check_stack_guard(),
            SandboxImpls::InitSingleUse(sbox) => sbox.check_stack_guard(),
            SandboxImpls::InitMultiUse(sbox) => sbox.check_stack_guard(),
        }
    }
}

impl From<hyperlight_host::SingleUseSandbox<'static>> for Sandbox {
    fn from(value: hyperlight_host::SingleUseSandbox<'static>) -> Self {
        Sandbox {
            should_recycle: false,
            inner: SandboxImpls::InitSingleUse(Box::new(value)),
        }
    }
}

impl From<hyperlight_host::MultiUseSandbox<'static>> for Sandbox {
    fn from(value: hyperlight_host::MultiUseSandbox<'static>) -> Self {
        Sandbox {
            should_recycle: true,
            inner: SandboxImpls::InitMultiUse(Box::new(value)),
        }
    }
}

impl AsRef<SandboxImpls> for Sandbox {
    fn as_ref(&self) -> &SandboxImpls {
        &self.inner
    }
}

impl AsMut<SandboxImpls> for Sandbox {
    fn as_mut(&mut self) -> &mut SandboxImpls {
        &mut self.inner
    }
}
