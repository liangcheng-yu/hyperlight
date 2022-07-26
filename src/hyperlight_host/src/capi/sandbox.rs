use super::context::Context;
use super::handle::Handle;
use super::strings::{to_string, RawCString};
use crate::sandbox::Sandbox;

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
    (*ctx).register_sandbox(sbox)
}
