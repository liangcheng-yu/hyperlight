use alloc::string::String;

use hyperlight_common::flatbuffer_wrappers::guest_error::ErrorCode;

pub type Result<T> = core::result::Result<T, HyperlightGuestError>;

#[derive(Debug)]
pub struct HyperlightGuestError {
    pub kind: ErrorCode,
    pub message: String,
}

impl HyperlightGuestError {
    pub fn new(kind: ErrorCode, message: String) -> Self {
        Self { kind, message }
    }
}
