use super::c_func::CFunc;
use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use crate::{capi::strings::get_string, sandbox::Sandbox};
use anyhow::Result;

/// Create a new `Sandbox` with the given guest binary to execute
/// and return a `Handle` reference to it.
///
/// # Safety
///
/// This function creates new memory on the heap, and it
/// is the caller's responsibility to free that memory when
/// it's no longer needed (but no sooner). Use `handle_free`
/// to do so.
#[no_mangle]
pub unsafe extern "C" fn sandbox_new(ctx: *mut Context, bin_path_hdl: Handle) -> Handle {
    CFunc::new("sandbox_new", ctx)
        .and_then_mut(|ctx, _| {
            let bin_path = get_string(&*ctx, bin_path_hdl)?;
            let sbox = Sandbox::new(bin_path.to_string());
            Ok(register_sandbox(ctx, sbox))
        })
        .ok_or_err_hdl()
}

/// Get a read-only reference to a `Sandbox` stored in `ctx` and
/// pointed to by `handle`.
pub fn get_sandbox(ctx: &Context, handle: Handle) -> Result<&Sandbox> {
    Context::get(handle, &ctx.sandboxes, |s| matches!(s, Hdl::Sandbox(_)))
}

fn register_sandbox(ctx: &mut Context, val: Sandbox) -> Handle {
    Context::register(val, &mut ctx.sandboxes, Hdl::Sandbox)
}

/// Get a read-and-write capable reference to a `Sandbox` stored in
/// `ctx` and pointed to by `handle`.
pub fn get_sandbox_mut(ctx: &mut Context, handle: Handle) -> Result<&mut Sandbox> {
    Context::get_mut(handle, &mut ctx.sandboxes, |s| matches!(s, Hdl::Sandbox(_)))
}
