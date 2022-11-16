use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::strings::{to_string, RawCString};
use crate::sandbox::Sandbox;
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
pub unsafe extern "C" fn sandbox_new(ctx: *mut Context, bin_path: RawCString) -> Handle {
    let bin_path = to_string(bin_path);
    let sbox = Sandbox::new(bin_path);
    Context::register(sbox, &mut (*ctx).sandboxes, Hdl::Sandbox)
}

/// Get a read-only reference to a `Sandbox` stored in `ctx` and
/// pointed to by `handle`.
pub fn get_sandbox(ctx: &Context, handle: Handle) -> Result<&Sandbox> {
    Context::get(handle, &ctx.sandboxes, |s| matches!(s, Hdl::Sandbox(_)))
}

/// Get a read-and-write capable reference to a `Sandbox` stored in
/// `ctx` and pointed to by `handle`.
pub fn get_sandbox_mut(ctx: &mut Context, handle: Handle) -> Result<&mut Sandbox> {
    Context::get_mut(handle, &mut ctx.sandboxes, |s| matches!(s, Hdl::Sandbox(_)))
}
