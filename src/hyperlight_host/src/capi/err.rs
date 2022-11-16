use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::strings::{to_c_string, to_string, RawCString};
use anyhow::{Error, Result};

/// Get the `anyhow::Error` stored in `ctx` referenced by `hdl`, if
/// one exists. If it does not, return `Err`.
pub fn get_err(ctx: &Context, hdl: Handle) -> Result<&Error> {
    Context::get(hdl, &ctx.errs, |h| matches!(h, Hdl::Err(_)))
}

/// Create a new `Handle` that references an error with the given message.
///
/// This function is unlikely to be useful in production code and is
/// provided for debug purposes.
///
/// If you pass `NULL` as `err_msg` or you pass a valid C-style string,
/// you will get back a `Handle` that is invalid and will not be usable
/// in any functions that expect a `Handle` as one of their arguments.
///
/// # Safety
///
/// You must call this function with a `Context *` that was:
///
/// - Created by `context_new`
/// - Not modified in any way besides via Hyperlight C API functions
/// - Not freed with `context_free`
#[no_mangle]
pub unsafe extern "C" fn handle_new_err(ctx: *mut Context, err_msg: RawCString) -> Handle {
    if err_msg.is_null() || libc::strlen(err_msg) == 0 {
        return Handle::new_invalid();
    }

    let msg_str = to_string(err_msg);
    let err = Error::msg(msg_str);
    (*ctx).register_err(err)
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
///
/// Additionally, this function creates new memory and transfers
/// ownership of it to the caller. Therefore, it is the caller's
/// responsibility to free the memory referenced by the returned
/// pointer by calling `error_message_free` exactly once after they're done
/// using it.
#[no_mangle]
pub unsafe extern "C" fn handle_get_error_message(ctx: *const Context, hdl: Handle) -> RawCString {
    match Context::get(hdl, &(*ctx).errs, |hdl| matches!(hdl, Hdl::Err(_))) {
        Ok(err) => to_c_string(err.to_string()).expect("error message couldn't be returned"),
        Err(_) => std::ptr::null(),
    }
}

/// Free the memory created by a handle_get_error_message call
#[no_mangle]
pub extern "C" fn error_message_free(_: Option<Box<RawCString>>) {}
