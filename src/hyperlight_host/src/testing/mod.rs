use std::fs;
use std::path::PathBuf;

use hyperlight_testing::rust_guest_as_pathbuf;

use crate::mem::exe::ExeInfo;
use crate::{new_error, Result};
pub(crate) mod log_values;

/// Get an `ExeInfo` representing `simpleguest.exe`
pub(crate) fn simple_guest_exe_info() -> Result<ExeInfo> {
    let bytes = bytes_for_path(rust_guest_as_pathbuf("simpleguest"))?;
    ExeInfo::from_buf(bytes.as_slice())
}

/// Get an `ExeInfo` representing `callbackguest.exe`
pub(crate) fn callback_guest_exe_info() -> Result<ExeInfo> {
    let bytes = bytes_for_path(rust_guest_as_pathbuf("callbackguest"))?;
    ExeInfo::from_buf(bytes.as_slice())
}

/// Read the file at `path_buf` into a `Vec<u8>` and return it,
/// or return `Err` if that went wrong
pub(crate) fn bytes_for_path(path_buf: PathBuf) -> Result<Vec<u8>> {
    let guest_path = path_buf
        .as_path()
        .to_str()
        .ok_or_else(|| new_error!("couldn't convert guest {:?} to a path", path_buf))?;
    let guest_bytes = fs::read(guest_path)
        .map_err(|e| new_error!("failed to open guest at path {} ({})", guest_path, e))?;
    Ok(guest_bytes)
}
