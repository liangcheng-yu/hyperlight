use super::{context::Context, handle::Handle, hdl::Hdl};
use crate::sandbox;
use anyhow::{bail, Result};

/// Either an initialized or uninitialized sandbox. This enum is used
/// to allow our `Sandbox` wrapper type to store both an uninitailized
/// or initialized sandbox at the same time.
pub(crate) enum EitherImpl {
    Uninit(Box<sandbox::UnintializedSandbox<'static>>),
    Init(Box<sandbox::Sandbox<'static>>),
}

/// This is the C API for the `Sandbox` type.
pub struct Sandbox {
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

    /// Consume `self`, store it inside `ctx`, then return a new `Handle`
    /// referencing it
    pub(crate) fn register(self, ctx: &mut Context) -> Handle {
        Context::register(self, &mut ctx.sandboxes, Hdl::Sandbox)
    }

    /// Consume `self`, check if it holds a `sandbox::UninitializedSandbox`,
    /// and return an immutable reference to it if so.
    /// Otherwise, return an `Err`
    pub(crate) fn to_uninit(&self) -> Result<&sandbox::UnintializedSandbox<'static>> {
        match &self.inner {
            EitherImpl::Uninit(sbox) => Ok(sbox),
            _ => bail!("attempted to get immutable uninitialzied sandbox from an initialized one"),
        }
    }
    /// Consume `self`, check if it holds a `sandbox::UninitializedSandbox`,
    /// and return an immutable reference to it if so.
    /// Otherwise, return an `Err`
    pub(crate) fn to_uninit_mut(&mut self) -> Result<&mut sandbox::UnintializedSandbox<'static>> {
        match &mut self.inner {
            EitherImpl::Uninit(sbox) => Ok(sbox),
            _ => bail!("attempted to get mutable uninitialzied sandbox from an initialized one"),
        }
    }
}

impl From<sandbox::Sandbox<'static>> for Sandbox {
    fn from(value: sandbox::Sandbox<'static>) -> Self {
        Sandbox {
            inner: EitherImpl::Init(Box::new(value)),
        }
    }
}

impl From<sandbox::UnintializedSandbox<'static>> for Sandbox {
    fn from(value: sandbox::UnintializedSandbox<'static>) -> Self {
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
