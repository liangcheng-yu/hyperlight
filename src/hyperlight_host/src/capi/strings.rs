use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use crate::validate_context_or_panic;
use anyhow::Result;
use std::ffi::{CStr, CString, NulError};
use std::os::raw::c_char;
use std::string::String;

/// A type alias for a common `NUL`-terminated C-style string.
pub type RawCString = *const c_char;

/// convert a RawCString into a String.
///
/// # Safety
///
/// This function "borrows" `c_string`,
/// makes a copy of the data, and then returns a
/// String representation of it. Since it does not take
/// ownership of `c_string`, you may still need to manually
/// free the memory to which `c_string` points. Whether you
/// do or don't need to do that depends on how you created
/// that memory.
pub unsafe fn to_string(c_string: RawCString) -> String {
    assert!(!c_string.is_null());
    CStr::from_ptr(c_string).to_string_lossy().into_owned()
}

/// Convert an Into<Vec<u8>> into a raw C string.
///
/// # Safety
///
/// If you use this function in your program, you must
/// follow a few rules to ensure you don't end up with
/// undefined behavior or memory problems:
///
/// - The returned memory must be freed manually by
/// calling `free_c_string`.
/// - `string` should not contain any null bytes in it. They
/// will be interpreted as the end of the string in C (and
/// `to_string` above), which can lead to memory leaks.
/// - If this function returns `Ok(cstr)`, you must not
/// modify `cstr` at all. If you do so, `free_c_string` may
/// not work properly.
pub fn to_c_string<T: Into<Vec<u8>>>(string: T) -> Result<RawCString, NulError> {
    CString::new(string).map(|s| s.into_raw() as RawCString)
}

/// Get the string value of the given `Handle`, or `NULL` if
/// `hdl` doesn't exist in `ctx` or it does exist but is not
/// a string value.
///
/// # Safety
///
/// `ctx` must have been created with `context_new` and must not be
/// modified or deleted while this function is running.
///
/// This function creates new memory. You must pass the returned
/// value to `free_raw_string()` after you're done using it.
#[no_mangle]
pub unsafe extern "C" fn handle_get_raw_string(ctx: *const Context, hdl: Handle) -> RawCString {
    validate_context_or_panic!(ctx);

    match Context::get(hdl, &(*ctx).strings, |s| matches!(s, Hdl::String(_))) {
        Ok(str) => match to_c_string((*str).clone()) {
            Ok(s) => s,
            Err(_) => std::ptr::null(),
        },
        Err(_) => std::ptr::null(),
    }
}

/// Free the memory created by a handle_get_raw_string call
#[no_mangle]
pub extern "C" fn free_raw_string(ptr: RawCString) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr as *mut c_char)) }
    }
}

/// Get a read-only reference to a string that is stored in `ctx`
/// and pointed to by `handle`.
pub fn get_string(ctx: &Context, handle: Handle) -> Result<&String> {
    Context::get(handle, &ctx.strings, |s| matches!(s, Hdl::String(_)))
}

/// Return true if the given handle `hdl` references a string in `ctx`,
/// and false otherwise
///
/// # Safety
///
/// `ctx` must be a valid pointer to a `Context` created with `context_new`,
/// owned by you, and not yet freed with `context_free`
#[no_mangle]
pub unsafe extern "C" fn handle_is_string(ctx: *const Context, hdl: Handle) -> bool {
    validate_context_or_panic!(ctx);
    get_string(&*ctx, hdl).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{to_c_string, to_string};

    #[test]
    fn string_roundtrip() {
        let orig = "STRING_ROUNDTRIP";
        let cstr = to_c_string(orig).unwrap();
        let str = unsafe { to_string(cstr) };
        assert_eq!(orig, str);
    }
}
