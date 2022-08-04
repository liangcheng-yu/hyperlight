use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::strings::{to_c_string, RawCString};

mod impls {
    use crate::capi::context::{Context, ReadResult};
    use crate::capi::handle::Handle;
    use crate::capi::hdl::Hdl;
    use anyhow::Error;
    /// Get the `anyhow::Error` stored in `ctx` referenced by `hdl`, if
    /// one exists. If it does not, return `Err`.
    pub fn get_err(ctx: &Context, hdl: Handle) -> ReadResult<Error> {
        Context::get(hdl, &ctx.errs, |h| matches!(h, Hdl::Err(_)))
    }
}

pub use impls::get_err;

/// Return true if `hdl` is an error type, false otherwise.
#[no_mangle]
pub extern "C" fn handle_is_error(hdl: Handle) -> bool {
    matches!(Hdl::try_from(hdl), Ok(Hdl::Err(_)))
}

/// Get the error message out of the given `Handle` or `NULL` if
/// `hdl` doesn't exist in `ctx` or it does exist but is not
/// an error.
///
/// # Safety
///
/// `ctx` must be a valid `Context*` created from `context_new` and owned
/// by the caller. It must not be modified or deleted while this
/// function is executing.
#[no_mangle]
pub unsafe extern "C" fn handle_get_error_message(ctx: *const Context, hdl: Handle) -> RawCString {
    match Context::get(hdl, &(*ctx).errs, |hdl| matches!(hdl, Hdl::Err(_))) {
        Ok(err) => to_c_string(err.to_string()).expect("error message couldn't be returned"),
        Err(_) => std::ptr::null(),
    }
}
