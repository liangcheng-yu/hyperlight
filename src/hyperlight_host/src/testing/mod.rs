use crate::mem::pe::pe_info::PEInfo;
use crate::new_error;
use crate::Result;
use hyperlight_testing::{callback_guest_as_pathbuf, rust_guest_as_pathbuf};
use std::fs;
use std::path::PathBuf;
pub(crate) mod log_values;
pub(crate) mod logger;
pub(crate) mod tracing_subscriber;

/// Get a `PEInfo` representing `simpleguest.exe`
pub(crate) fn simple_guest_pe_info() -> Result<PEInfo> {
    let bytes = bytes_for_path(rust_guest_as_pathbuf("simpleguest"))?;
    PEInfo::new(bytes.as_slice())
}

/// Get a `PEInfo` representing `callbackguest.exe`
pub(crate) fn callback_guest_pe_info() -> Result<PEInfo> {
    let bytes = bytes_for_path(callback_guest_as_pathbuf())?;
    PEInfo::new(bytes.as_slice())
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
