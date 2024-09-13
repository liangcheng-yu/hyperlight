use alloc::format;
use alloc::string::String;

use hyperlight_common::flatbuffer_wrappers::guest_error::ErrorCode;
use {anyhow, serde_json};

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

impl From<anyhow::Error> for HyperlightGuestError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            kind: ErrorCode::GuestError,
            message: format!("Error: {:?}", error),
        }
    }
}

impl From<serde_json::Error> for HyperlightGuestError {
    fn from(error: serde_json::Error) -> Self {
        Self {
            kind: ErrorCode::GuestError,
            message: format!("Error: {:?}", error),
        }
    }
}
