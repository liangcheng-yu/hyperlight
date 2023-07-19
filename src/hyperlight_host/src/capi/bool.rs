use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use crate::validate_context_or_panic;
use anyhow::Result;

/// Return true if the given handle `hdl` references a boolean in `ctx`,
/// and false otherwise
///
/// # Safety
///
/// `ctx` must be a valid pointer to a `Context` created with `context_new`,
/// owned by you, and not yet freed with `context_free`
#[no_mangle]
pub unsafe extern "C" fn handle_is_boolean(ctx: *const Context, hdl: Handle) -> bool {
    validate_context_or_panic!(ctx);
    get_boolean(&*ctx, hdl).is_ok()
}

/// Return the value of the boolean in `ctx` referenced by `hdl`. If
/// `hdl` does not reference a boolean in `ctx`, the return value is
/// undefined. Call `handle_is_boolean` to validate `hdl` before calling
/// this funciton.
///
/// # Safety
///
/// `ctx` must be a valid pointer to a `Context` created with `context_new`,
/// owned by you, and not yet freed with `context_free`
#[no_mangle]
pub unsafe extern "C" fn handle_get_boolean(ctx: *const Context, hdl: Handle) -> bool {
    validate_context_or_panic!(ctx);
    get_boolean(&*ctx, hdl).unwrap_or(false)
}

fn get_boolean(ctx: &Context, hdl: Handle) -> Result<bool> {
    Context::get(hdl, &ctx.booleans, |hdl| matches!(hdl, Hdl::Boolean(_))).map(|b| *b)
}

/// Store `val` in `ctx` and return a new `Handle` referencing it
pub(crate) fn register_boolean(ctx: &mut Context, val: bool) -> Handle {
    Context::register(val, &mut ctx.booleans, Hdl::Boolean)
}
