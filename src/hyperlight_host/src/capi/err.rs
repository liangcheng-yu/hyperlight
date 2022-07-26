use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::strings::{to_c_string, RawCString};

/// Return true if `hdl` is an error type, false otherwise.
#[no_mangle]
pub extern "C" fn handle_is_error(hdl: Handle) -> bool {
    matches!(Hdl::try_from(hdl), Ok(Hdl::Err(_)))
}

/// Print the error referenced by `hdl`, if `hdl` references a valid
/// error within `ctx`.
///
/// # Safety
///
/// `ctx` must be a valid `Context*` created from `context_new` and owned
/// by the caller. It must not be modified or deleted while this
/// function is executing.must
#[no_mangle]
pub unsafe extern "C" fn handle_print_error(ctx: *const Context, hdl: Handle) -> bool {
    match (*ctx).get_err(hdl) {
        Ok(e) => {
            println!("ERROR: {}", *e);
            true
        }
        _ => false,
    }
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
    match (*ctx).get_err(hdl) {
        Ok(err) => to_c_string(err.to_string()).expect("error message couldn't be returned"),
        Err(_) => std::ptr::null(),
    }
}

/// Print the error referenced by `hdl`, if one exists, to STDERR.
///
/// Returns `true` if `hdl` references an error in `ctx`, `false`
/// otherwise.
///
/// # Safety
///
/// `ctx` must be a valid `Context*` created from `context_new` and owned
/// by the caller. It must not be modified or deleted while this
/// function is executing.
#[no_mangle]
pub unsafe extern "C" fn handle_print_error_message(ctx: *const Context, hdl: Handle) -> bool {
    match (*ctx).get_err(hdl) {
        Ok(err) => {
            eprintln!("ERROR: {}", *err);
            true
        }
        Err(_) => false,
    }
}
