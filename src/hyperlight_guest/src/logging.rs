use alloc::string::ToString;
use alloc::vec::Vec;

use hyperlight_common::flatbuffer_wrappers::guest_log_data::GuestLogData;
use hyperlight_common::flatbuffer_wrappers::guest_log_level::LogLevel;

use crate::host_function_call::{outb, OutBAction};
use crate::shared_output_data::push_shared_output_data;

fn write_log_data(
    log_level: LogLevel,
    message: &str,
    source: &str,
    caller: &str,
    source_file: &str,
    line: u32,
) {
    let guest_log_data = GuestLogData::new(
        message.to_string(),
        source.to_string(),
        log_level,
        caller.to_string(),
        source_file.to_string(),
        line,
    );

    let bytes: Vec<u8> = guest_log_data
        .try_into()
        .expect("Failed to convert GuestLogData to bytes");

    push_shared_output_data(bytes).expect("Unable to push log data to shared output data");
}

pub fn log_message(
    log_level: LogLevel,
    message: &str,
    source: &str,
    caller: &str,
    source_file: &str,
    line: u32,
) {
    write_log_data(log_level, message, source, caller, source_file, line);
    outb(OutBAction::Log as u16, 0);
}
