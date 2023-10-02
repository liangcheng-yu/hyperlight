use super::log_level::LogLevel;
use crate::flatbuffers::hyperlight::generated::size_prefixed_root_as_guest_log_data;
use crate::flatbuffers::hyperlight::generated::{
    GuestLogData as GuestLogDataFb, GuestLogDataArgs, LogLevel as LogLevelFb,
};
use crate::mem::{layout::SandboxMemoryLayout, shared_mem::SharedMemory};
use anyhow::{anyhow, bail, Error, Result};
use std::mem::size_of;

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
    #[cfg(test)]
    pub(crate) fn new(
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
    /// Write `self` to the appropriate location in `shared_mem`, as
    /// defined by `layout`. Return `Ok` if the write operation succeeded,
    /// and `Err` otherwise.
    ///
    /// This method is never used by production Rust code. Writes only
    /// happen from the guest side (which is written in C). It is used
    /// in test code, however, thus it's compiled for tests only
    #[cfg(test)]
    pub(crate) fn write_to_memory(
        &self,
        shared_mem: &mut SharedMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<()> {
        let guest_log_data_buffer: Vec<u8> = self.try_into()?;
        shared_mem.copy_from_slice(
            guest_log_data_buffer.as_slice(),
            layout.get_output_data_offset(),
        )
    }
}

impl TryFrom<Vec<u8>> for GuestLogData {
    type Error = Error;
    fn try_from(raw_vec: Vec<u8>) -> Result<Self> {
        Self::try_from(raw_vec.as_slice())
    }
}

impl TryFrom<&[u8]> for GuestLogData {
    type Error = Error;
    fn try_from(raw_bytes: &[u8]) -> Result<Self> {
        let gld_gen = size_prefixed_root_as_guest_log_data(raw_bytes).map_err(|e| anyhow!(e))?;
        let message = convert_generated_option("message", gld_gen.message())?;
        let source = convert_generated_option("source", gld_gen.source())?;
        let level = LogLevel::try_from(gld_gen.level())?;
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

impl TryFrom<(&SharedMemory, SandboxMemoryLayout)> for GuestLogData {
    type Error = Error;
    fn try_from(value: (&SharedMemory, SandboxMemoryLayout)) -> Result<Self> {
        let (shared_mem, layout) = value;
        let offset = layout.get_output_data_offset();
        // there's a u32 at the beginning of the GuestLogData
        // with the size
        let size = shared_mem.read_u32(offset)?;
        // read size + 32 bits from shared memory, starting at
        // layout.get_output_data_offset
        let mut vec_out = {
            let len_usize = usize::try_from(size)? + size_of::<u32>();
            vec![0; len_usize]
        };
        shared_mem.copy_to_slice(vec_out.as_mut_slice(), offset)?;
        GuestLogData::try_from(vec_out.as_slice())
    }
}

impl TryFrom<&GuestLogData> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: &GuestLogData) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let message = builder.create_string(&value.message);
        let source = builder.create_string(&value.source);
        let caller = builder.create_string(&value.caller);
        let source_file = builder.create_string(&value.source_file);
        let level = LogLevelFb::from(&value.level);

        let guest_log_data_fb = GuestLogDataFb::create(
            &mut builder,
            &GuestLogDataArgs {
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
            bail!("The capacity of the vector is for GuestLogData is incorrect");
        }

        Ok(res)
    }
}

impl TryFrom<GuestLogData> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: GuestLogData) -> Result<Vec<u8>> {
        (&value).try_into()
    }
}

fn convert_generated_option(field_name: &str, opt: Option<&str>) -> Result<String> {
    opt.map(|s| s.to_string())
        .ok_or_else(|| Error::msg(format!("no {field_name} found in decoded GuestLogData")))
}

#[cfg(test)]
mod test {
    use super::GuestLogData;
    use crate::{
        func::guest::log_level::LogLevel,
        mem::{layout::SandboxMemoryLayout, shared_mem::SharedMemory},
        sandbox::SandboxConfiguration,
    };

    #[test]
    fn round_trip() {
        let gld = GuestLogData {
            message: "test message".to_string(),
            source: "test source".to_string(),
            caller: "test caller".to_string(),
            source_file: "test source file".to_string(),
            line: 123,
            level: LogLevel::Critical,
        };
        let decoded = {
            // copy the encoded Vec<u8> into shared memory so we can
            // turn around and pull the bytes out and try to decode
            let layout =
                SandboxMemoryLayout::new(SandboxConfiguration::default(), 12, 12, 12).unwrap();
            let mut shared_mem = SharedMemory::new(layout.get_memory_size().unwrap()).unwrap();
            gld.write_to_memory(&mut shared_mem, &layout).unwrap();
            GuestLogData::try_from((&shared_mem, layout)).unwrap()
        };
        assert_eq!(gld, decoded);
    }
}
