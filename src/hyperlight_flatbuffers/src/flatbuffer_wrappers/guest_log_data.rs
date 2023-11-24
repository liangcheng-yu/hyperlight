use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use anyhow::{anyhow, bail, Error, Result};
use tracing::{instrument, Span};

use crate::flatbuffers::hyperlight::generated::{
    size_prefixed_root_as_guest_log_data, GuestLogData as FbGuestLogData,
    GuestLogDataArgs as FbGuestLogDataArgs, LogLevel as FbLogLevel,
};

use super::guest_log_level::LogLevel;

/// The guest log data for a VM sandbox
#[derive(Eq, PartialEq, Debug, Clone)]
#[allow(missing_docs)]
pub struct GuestLogData {
    pub message: String,
    pub source: String,
    pub level: LogLevel,
    pub caller: String,
    pub source_file: String,
    pub line: u32,
}

impl GuestLogData {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn new(
        message: String,
        source: String,
        level: LogLevel,
        caller: String,
        source_file: String,
        line: u32,
    ) -> Self {
        Self {
            message,
            source,
            level,
            caller,
            source_file,
            line,
        }
    }
}

impl TryFrom<&[u8]> for GuestLogData {
    type Error = Error;
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn try_from(raw_bytes: &[u8]) -> Result<Self> {
        let gld_gen = size_prefixed_root_as_guest_log_data(raw_bytes).unwrap();
        let message = convert_generated_option("message", gld_gen.message())?;
        let source = convert_generated_option("source", gld_gen.source())?;
        let level = LogLevel::try_from(&gld_gen.level())?;
        let caller = convert_generated_option("caller", gld_gen.caller())?;
        let source_file = convert_generated_option("source file", gld_gen.source_file())?;
        let line = gld_gen.line();

        Ok(GuestLogData {
            message,
            source,
            level,
            caller,
            source_file,
            line,
        })
    }
}

impl TryFrom<&GuestLogData> for Vec<u8> {
    type Error = Error;
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn try_from(value: &GuestLogData) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let message = builder.create_string(&value.message);
        let source = builder.create_string(&value.source);
        let caller = builder.create_string(&value.caller);
        let source_file = builder.create_string(&value.source_file);
        let level = FbLogLevel::from(&value.level);

        let guest_log_data_fb = FbGuestLogData::create(
            &mut builder,
            &FbGuestLogDataArgs {
                message: Some(message),
                source: Some(source),
                level,
                caller: Some(caller),
                source_file: Some(source_file),
                line: value.line,
            },
        );
        builder.finish_size_prefixed(guest_log_data_fb, None);
        let res = builder.finished_data().to_vec();

        // This vector may be converted to a raw pointer and returned via the C API and the C API uses the size prefix to determine the capacity and length of the buffer in order to free the memory  , therefore:
        // 1. the capacity of the vector should be the same as the length
        // 2. the capacity of the vector should be the same as the size of the buffer (frm the size prefix) + 4 bytes (the size of the size prefix field is not included in the size)

        let length = unsafe { flatbuffers::read_scalar::<i32>(&res[..4]) };

        if res.capacity() != res.len() || res.capacity() != length as usize + 4 {
            bail!(
                "The capacity of the vector ({}) is not the same as the length ({}) or the size ({}) of the buffer", 
                res.capacity(),
                length,
                res.len()
            )
        }

        Ok(res)
    }
}

impl TryFrom<GuestLogData> for Vec<u8> {
    type Error = Error;
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn try_from(value: GuestLogData) -> Result<Vec<u8>> {
        (&value).try_into()
    }
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
fn convert_generated_option(field_name: &str, opt: Option<&str>) -> Result<String> {
    opt.map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Missing field: {}", field_name))
}
