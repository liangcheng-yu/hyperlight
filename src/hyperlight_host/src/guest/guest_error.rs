extern crate flatbuffers;
use crate::flatbuffers::hyperlight::generated::{
    size_prefixed_root_as_guest_error, ErrorCode, GuestError as GuestErrorFb, GuestErrorArgs,
};
use crate::mem::layout::SandboxMemoryLayout;
use crate::mem::shared_mem::SharedMemory;
use anyhow::{anyhow, bail, Result};
use readonly;
use std::convert::{TryFrom, TryInto};

/// The error code of a `GuestError`.
pub type Code = ErrorCode;

/// `GuestError` represents an error taht occurred in the Hyperlight Guest.
#[derive(Debug, Clone)]
#[readonly::make]
pub struct GuestError {
    /// The error code.
    pub code: Code,
    /// The error message.
    pub message: String,
}

impl GuestError {
    /// Create a new GuestError.
    pub fn new(code: Code, message: String) -> Self {
        Self { code, message }
    }

    fn get_memory_buffer_max_size(
        guest_mem: &SharedMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<u64> {
        let err_buffer_size_offset = layout.get_guest_error_buffer_size_offset();
        let max_err_buffer_size = guest_mem.read_u64(err_buffer_size_offset)?;
        Ok(max_err_buffer_size)
    }

    /// Write the guest error to the shared memory.
    pub fn write_to_memory(
        self,
        guest_mem: &mut SharedMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<()> {
        let guest_error_buffer: Vec<u8> = self.try_into()?;
        let max_error_buffer_size = Self::get_memory_buffer_max_size(guest_mem, layout)?;
        if guest_error_buffer.len() as u64 > max_error_buffer_size {
            bail!("The guest error message is too large to fit in the shared memory");
        }
        guest_mem.copy_from_slice(
            guest_error_buffer.as_slice(),
            layout.guest_error_buffer_offset,
        )?;
        Ok(())
    }
}

impl TryFrom<(&SharedMemory, &SandboxMemoryLayout)> for GuestError {
    type Error = anyhow::Error;
    fn try_from(value: (&SharedMemory, &SandboxMemoryLayout)) -> Result<Self> {
        let max_err_buffer_size = Self::get_memory_buffer_max_size(value.0, value.1)?;
        let mut guest_error_buffer = vec![b'0'; usize::try_from(max_err_buffer_size)?];
        let err_msg_offset = value.1.guest_error_buffer_offset;
        value
            .0
            .copy_to_slice(guest_error_buffer.as_mut_slice(), err_msg_offset)?;
        GuestError::try_from(guest_error_buffer.as_slice())
    }
}

impl TryFrom<&[u8]> for GuestError {
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        let guest_error_fb = size_prefixed_root_as_guest_error(value).map_err(|e| anyhow!(e))?;
        let code = guest_error_fb.code();
        let message = match guest_error_fb.message() {
            Some(message) => message.to_string(),
            None => String::new(),
        };
        Ok(Self { code, message })
    }
}

impl TryFrom<&GuestError> for Vec<u8> {
    type Error = anyhow::Error;
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
            anyhow::bail!("The capacity of the vector is for GuestError is incorrect");
        }

        Ok(res)
    }
}

impl TryFrom<GuestError> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: GuestError) -> Result<Vec<u8>> {
        (&value).try_into()
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
