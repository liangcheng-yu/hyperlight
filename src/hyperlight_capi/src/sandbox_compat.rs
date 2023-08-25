extern crate hyperlight_host;
use super::{context::Context, handle::Handle, hdl::Hdl};
use anyhow::{anyhow, bail, Result};
use hyperlight_host::sandbox::mem_mgr::MemMgrWrapperGetter;
use hyperlight_host::UninitializedSandbox;

/// Either an initialized or uninitialized sandbox. This enum is used
/// to allow our `Sandbox` wrapper type to store both an uninitailized
/// or initialized sandbox at the same time.
pub(crate) enum EitherImpl {
    Uninit(Box<hyperlight_host::sandbox::uninitialized::UninitializedSandbox<'static>>),
    Init(Box<hyperlight_host::sandbox::initialized::Sandbox<'static>>),
}

/// This is the C API for the `Sandbox` type.
pub(crate) struct Sandbox {
    inner: EitherImpl,
}

impl Sandbox {
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

    /// Find the `Sandbox` in `ctx` referenced by `hdl`. If it was found,
    /// remove the `EitherImpl` from it. Then, pass that `EitherImpl` to
    /// `cb_fn`. If `cb_fn` returns an `Ok`, set the new `EitherImpl` value
    /// to the `Sandbox`'s inner value and re-insert the `Sandbox` into `ctx`
    /// with the same `Handle` `hdl`. If anything went wrong along the way,
    /// return an `Err`. If an error occurred and the `Sandbox` was already
    /// removed from `ctx`, do not re-insert it into `ctx`.
    pub(crate) fn replace<F>(ctx: &mut Context, hdl: Handle, cb_fn: F) -> Result<()>
    where
        F: FnOnce(EitherImpl) -> Result<EitherImpl>,
    {
        let mut sbox = ctx
            .sandboxes
            .remove(&hdl.key())
            .ok_or(anyhow!("no sandbox exists for the given handle"))?;
        let new_impl = cb_fn(sbox.inner)?;
        sbox.inner = new_impl;
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
            EitherImpl::Uninit(sbox) => Ok(sbox),
            _ => bail!("attempted to get immutable uninitialzied sandbox from an initialized one"),
        }
    }
    /// Consume `self`, check if it holds a `sandbox::UninitializedSandbox`,
    /// and return an immutable reference to it if so.
    /// Otherwise, return an `Err`
    pub(crate) fn to_uninit_mut(&mut self) -> Result<&mut UninitializedSandbox<'static>> {
        match &mut self.inner {
            EitherImpl::Uninit(sbox) => Ok(sbox),
            _ => bail!("attempted to get mutable uninitialzied sandbox from an initialized one"),
        }
    }

    pub(crate) fn check_stack_guard(&self) -> Result<bool> {
        match &self.inner {
            EitherImpl::Uninit(sbox) => sbox.get_mem_mgr_wrapper().check_stack_guard(),
            EitherImpl::Init(sbox) => sbox.get_mem_mgr_wrapper().check_stack_guard(),
        }
    }
}

impl From<hyperlight_host::sandbox::initialized::Sandbox<'static>> for Sandbox {
    fn from(value: hyperlight_host::sandbox::initialized::Sandbox<'static>) -> Self {
        Sandbox {
            inner: EitherImpl::Init(Box::new(value)),
        }
    }
}

impl From<hyperlight_host::sandbox::uninitialized::UninitializedSandbox<'static>> for Sandbox {
    fn from(value: hyperlight_host::sandbox::uninitialized::UninitializedSandbox<'static>) -> Self {
        Sandbox {
            inner: EitherImpl::Uninit(Box::new(value)),
        }
    }
}

impl AsRef<EitherImpl> for Sandbox {
    fn as_ref(&self) -> &EitherImpl {
        &self.inner
    }
}

impl AsMut<EitherImpl> for Sandbox {
    fn as_mut(&mut self) -> &mut EitherImpl {
        &mut self.inner
    }
}
