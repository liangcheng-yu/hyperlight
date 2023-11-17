extern crate flatbuffers;

use crate::flatbuffers::hyperlight::generated::{
    size_prefixed_root_as_guest_error, ErrorCode, GuestError as GuestErrorFb, GuestErrorArgs,
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

/// The error code of a `GuestError`.
pub type Code = ErrorCode;

/// `GuestError` represents an error that occurred in the Hyperlight Guest.
#[derive(Debug, Clone)]
pub struct GuestError {
    /// The error code.
    pub code: Code,
    /// The error message.
    pub message: String,
}

impl GuestError {
    pub fn new(code: Code, message: String) -> Self {
        Self { code, message }
    }
}

impl TryFrom<&[u8]> for GuestError {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        let guest_error_fb = size_prefixed_root_as_guest_error(value)?;
        let code = guest_error_fb.code();
        let message = match guest_error_fb.message() {
            Some(message) => message.to_string(),
            None => String::new(),
        };
        Ok(Self { code, message })
    }
}

impl TryFrom<&GuestError> for Vec<u8> {
    type Error = Error;
    fn try_from(value: &GuestError) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let message = builder.create_string(&value.message);

        let guest_error_fb = GuestErrorFb::create(
            &mut builder,
            &GuestErrorArgs {
                code: value.code,
                message: Some(message),
            },
        );
        builder.finish_size_prefixed(guest_error_fb, None);
        let res = builder.finished_data().to_vec();

        // This vector may be converted to a raw pointer and returned via the C API and the C API uses the size prefix to determine the capacity and length of the buffer in order to free the memory  , therefore:
        // 1. the capacity of the vector should be the same as the length
        // 2. the capacity of the vector should be the same as the size of the buffer (frm the size prefix) + 4 bytes (the size of the size prefix field is not included in the size)

        let length = unsafe { flatbuffers::read_scalar::<i32>(&res[..4]) };

        if res.capacity() != res.len() || res.capacity() != length as usize + 4 {
            bail!(
                "VectorCapacityInCorrect {} {} {}",
                res.capacity(),
                res.len(),
                length + 4
            );
        }

        Ok(res)
    }
}

impl Default for GuestError {
    fn default() -> Self {
        Self {
            code: Code::NoError,
            message: String::new(),
        }
    }
}
