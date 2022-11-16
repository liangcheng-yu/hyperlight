use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;

mod impls {
    use crate::capi::context::Context;
    use crate::capi::handle::Handle;
    use crate::capi::sandbox::get_sandbox;
    use anyhow::Result;

    /// Get the guest binary path from the sandbox stored in `ctx`
    /// and referenced by `sbox_hdl`.
    ///
    /// Returns `Ok` if `sbox_hdl` is valid, `Err` otherwise.
    pub fn guest_binary_path(ctx: &Context, sbox_hdl: Handle) -> Result<String> {
        match get_sandbox(ctx, sbox_hdl) {
            Ok(sbox) => Ok(sbox.bin_path.clone()),
            Err(err) => Err(err),
        }
    }

    /// Determine whether the hypervisor is present, as reported by
    /// the sandbox stored in `ctx` and referenced by `sbox_hdl`.
    ///
    /// Returns `Ok` if `sbox_hdl` is valid and it could be determined
    /// whether the hypervisor was or wasn't present, and `Err` otherwise.
    pub fn is_hypervisor_present(ctx: &Context, sbox_hdl: Handle) -> Result<bool> {
        match get_sandbox(ctx, sbox_hdl) {
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
        Ok(path) => Context::register(path, &mut (*ctx).strings, Hdl::String),
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
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C
/// API
#[no_mangle]
pub unsafe extern "C" fn is_hypervisor_present(ctx: *const Context, sbox_hdl: Handle) -> bool {
    impls::is_hypervisor_present(&(*ctx), sbox_hdl).unwrap_or(false)
}
