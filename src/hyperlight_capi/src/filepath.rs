use super::strings::{to_c_string, to_string, RawCString};

fn path_sep() -> char {
    cfg_if::cfg_if! {
        if #[cfg(unix)] {
            '/'
        } else {
            '\\'
        }
    }
}

/// join path1 and path2 together as a complete path using the appropriate
/// path separator for the platform.
///
/// # Safety
///
/// `path1` and `path2` must point to valid NUL-terminated C-strings that
/// you own. They must not be modified or deleted for the duration
/// of this function's execution.
///
/// The caller of this function owns the memory referenced by the
/// return value. It is their responsibility to release the memory
/// when they're done with it by passing the pointer to `free` exactly
/// once.
#[no_mangle]
pub unsafe extern "C" fn file_path_join(path1: RawCString, path2: RawCString) -> RawCString {
    let path1_str = to_string(path1);
    let path2_str = to_string(path2);

    match to_c_string(path1_str + path_sep().to_string().as_str() + path2_str.as_str()) {
        Ok(s) => s,
        _ => std::ptr::null(),
    }
}
