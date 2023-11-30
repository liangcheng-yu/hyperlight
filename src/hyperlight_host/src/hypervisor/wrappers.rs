use crate::{HyperlightError, Result};
use std::ffi::CString;
use tracing::{instrument, Span};
use windows::core::PSTR;

/// A wrapper for `windows::core::PSTR` values that ensures memory for the
/// underlying string is properly dropped.
#[derive(Debug)]
pub(super) struct PSTRWrapper(*mut i8);

impl TryFrom<&str> for PSTRWrapper {
    type Error = HyperlightError;
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn try_from(value: &str) -> Result<Self> {
        let c_str = CString::new(value)?;
        Ok(Self(c_str.into_raw()))
    }
}

impl Drop for PSTRWrapper {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn drop(&mut self) {
        let cstr = unsafe { CString::from_raw(self.0) };
        drop(cstr);
    }
}

/// Convert a `WindowsStringWrapper` into a `PSTR`.
///
/// # Safety
/// The returned `PSTR` must not outlive the origin `WindowsStringWrapper`
impl From<&PSTRWrapper> for PSTR {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn from(value: &PSTRWrapper) -> Self {
        let raw = value.0;
        PSTR::from_raw(raw as *mut u8)
    }
}
