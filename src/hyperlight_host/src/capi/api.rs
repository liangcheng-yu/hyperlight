use super::c_func::CFunc;
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
    CFunc::new("guest_binary_path", ctx)
        .and_then(|c, _| impls::guest_binary_path(c, sbox_hdl))
        .map_mut(|c, path| Context::register(path, &mut c.strings, Hdl::String))
        .ok_or_err_hdl()
}
