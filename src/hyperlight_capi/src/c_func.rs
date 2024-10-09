use anyhow::anyhow;
use hyperlight_host::{new_error, Result};
use tracing::{error, info_span, trace};

use super::context::Context;
use super::handle::Handle;

/// Roughly equivalent to a `Result` type with the following additional
/// features:
///
/// - Keeps extra state about the "function name" currently being executed,
///   and adds that information to error messages
/// - Holds a `*mut Context` and automatically includes it in the parameter
///   list of all `map`, `and_then`, etc... functions
/// - Ensures that the `*mut Context` is non-null, and automatically
///   returns an error or panics, as appropriate, if it is
pub(super) struct CFunc<T> {
    func_name: String,
    ctx: *mut Context,
    other: Result<T>,
}

impl CFunc<()> {
    /// Create a new instance of `Self` with the given `func_name` and
    /// `Context` pointer.
    ///
    /// Given an FFI function that looks similar to the following:
    ///
    /// ```rust
    /// pub extern "C" fn my_ffi_func(ctx: *mut Context)
    /// ```
    ///
    /// `func_name` should be passed as `"my_ffi_func"` (i.e. the same
    /// name as the function in which the `CFunc` is being created) and
    /// `ctx` should be passed as the same `ctx` as `my_ffi_func` accepts
    /// as a parameter.
    pub(super) fn new(func_name: &str, ctx: *mut Context) -> CFunc<()> {
        CFunc {
            func_name: func_name.to_string(),
            ctx,
            other: Ok(()),
        }
    }
}

impl<T> CFunc<T> {
    /// Consume `self`, then run `run_fn(ctx_ref, other_val)` if the following
    /// conditions exist:
    ///
    /// - The internally stored `Context` pointer is non-`NULL`
    /// - The internally stored "other" value is `Ok`
    ///
    /// Pass the a `&mut Context` as the `ctx_ref` parameter and
    /// the other parameter as `other_val`, then return a new `CFunc` with
    /// the result of the call to `run_fn`
    pub(super) fn and_then_mut<RunFn, RunRes>(self, run_fn: RunFn) -> CFunc<RunRes>
    where
        RunFn: FnOnce(&mut Context, T) -> Result<RunRes>,
    {
        let ctx_res = if self.ctx.is_null() {
            Err(new_error!("{}: NULL context passed", self.func_name))
        } else {
            unsafe { Ok(&mut *self.ctx) }
        };
        CFunc {
            other: ctx_res.and_then(|ctx| {
                let span = info_span!(
                    "function_call",
                    FunctionName = self.func_name,
                    CorrelationId = ctx.correlation_id
                )
                .entered();
                self.other.and_then(|other| {
                    trace!(
                        CorrelationId = ctx.correlation_id,
                        "Calling Function {}",
                        self.func_name
                    );
                    let res = run_fn(ctx, other);
                    trace!(
                        CorrelationId = ctx.correlation_id,
                        "Returned from Calling Function {}",
                        self.func_name
                    );
                    if let Err(ref e) = res {
                        error!(
                            FunctionName = self.func_name,
                            CorrelationId = ctx.correlation_id,
                            Error = e.to_string()
                        );
                    }
                    span.exit();
                    res
                })
            }),
            func_name: self.func_name,
            ctx: self.ctx,
        }
    }

    /// Equivalent to `and_then_mut` except only used for callback functions
    /// that take `&Context`, rather than `&mut Context`
    pub(super) fn and_then<RunFn, RunRes>(self, run_fn: RunFn) -> CFunc<RunRes>
    where
        RunFn: FnOnce(&Context, T) -> Result<RunRes>,
    {
        self.and_then_mut(|c, t| run_fn(c, t))
    }
}

impl CFunc<Handle> {
    pub(super) fn ok_or_err_hdl(self) -> Handle {
        let ctx_res = if self.ctx.is_null() {
            Err(anyhow!("{}: NULL context passed", self.func_name))
        } else {
            unsafe { Ok(&mut *self.ctx) }
        };

        match (ctx_res, self.other) {
            // valid context, valid other - just return other
            (Ok(_), Ok(other)) => other,
            // valid context, invalid other - call ctx.register_err
            (Ok(ctx), Err(e)) => ctx.register_err(e),
            // invalid context, regardless of other - return NULL_CONTEXT_HANDLE
            (Err(_), _) => Handle::new_null_context(),
        }
    }
}
