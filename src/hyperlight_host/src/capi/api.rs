use super::context::Context;
use super::handle::Handle;

mod impls {
    use crate::capi::context::Context;
    use crate::capi::handle::Handle;
    use anyhow::Result;
    pub fn guest_binary_path(ctx: &Context, sbox_hdl: Handle) -> Result<String> {
        match ctx.get_sandbox_mut(sbox_hdl) {
            Ok(sbox) => Ok(sbox.bin_path.clone()),
            Err(err) => Err(err),
        }
    }

    pub fn is_hypervisor_present(ctx: &Context, sbox_hdl: Handle) -> Result<bool> {
        match ctx.get_sandbox(sbox_hdl) {
            Ok(sbox) => sbox.is_hypervisor_present(),
            Err(e) => Err(e),
        }
    }
}

/// Returns a `Handle` for the path to the binary that the given
/// sandbox handle will (and/or has) run, or a `Handle` to an error
/// if there was an issue.
///
/// # Safety
///
/// This function creates new memory for the returned `Handle`, and
/// you must free that memory by calling `handle_free` exactly once
/// after you're done using it.
#[no_mangle]
pub unsafe extern "C" fn guest_binary_path(ctx: *mut Context, sbox_hdl: Handle) -> Handle {
    match impls::guest_binary_path(&(*ctx), sbox_hdl) {
        Ok(path) => (*ctx).register_string(path),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Determine whether the hypervisor for use with
/// the given sandbox handle is presently available for use.
///
/// Returns false if:
///
/// - `sbox_hdl` points to a valid sandbox and the machine on which
/// this code is running does not have a compatible hypervisor
/// available for use.
/// - `sbox_hdl` points to an invalid sandbox
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C
/// API
#[no_mangle]
pub unsafe extern "C" fn is_hypervisor_present(ctx: *const Context, sbox_hdl: Handle) -> bool {
    impls::is_hypervisor_present(&(*ctx), sbox_hdl).unwrap_or(false)
}
