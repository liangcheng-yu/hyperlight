use std::ffi::CString;
use windows::core::PSTR;

use crate::{HyperlightError, Result};

/// A wrapper for `windows::core::PSTR` values that ensures memory for the
/// underlying string is properly dropped.
pub(super) struct PSTRWrapper(*mut i8);

impl TryFrom<&str> for PSTRWrapper {
    type Error = HyperlightError;
    fn try_from(value: &str) -> Result<Self> {
        let c_str = CString::new(value)?;
        Ok(Self(c_str.into_raw()))
    }
}

impl Drop for PSTRWrapper {
    fn drop(&mut self) {
        unsafe { CString::from_raw(self.0) };
    }
}

/// Convert a `WindowsStringWrapper` into a `PSTR`.
///
/// # Safety
/// The returned `PSTR` must not outlive the origin `WindowsStringWrapper`
impl From<&PSTRWrapper> for PSTR {
    fn from(value: &PSTRWrapper) -> Self {
        let raw = value.0;
        PSTR::from_raw(raw as *mut u8)
    }
}
