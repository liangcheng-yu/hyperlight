use core::ptr::copy_nonoverlapping;

use alloc::{string::ToString, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    guest_log_data::GuestLogData, guest_log_level::LogLevel,
};

use crate::{
    host_function_call::{outb, OutBAction},
    P_PEB,
};

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

    unsafe {
        let peb_ptr = P_PEB.unwrap();
        let output_data_buffer = (*peb_ptr).outputdata.outputDataBuffer as *mut u8;

        copy_nonoverlapping(bytes.as_ptr(), output_data_buffer, bytes.len());
    }
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

#[macro_export]
macro_rules! log {
    ($log_level:expr, $message:expr) => {{
        $crate::logging::log_message(
            $log_level,
            $message,
            module_path!(),
            // there is no way to defnitively get the caller name
            "Unknown",
            file!(),
            line!(),
        )
    }};
}

#[macro_export]
macro_rules! info {
    ($message:expr) => {{
        $crate::logging::log_message(
            hyperlight_flatbuffers::flatbuffer_wrappers::guest_log_level::LogLevel::Information,
            $message,
            module_path!(),
            // there is no way to defnitively get the caller name
            "Unknown",
            file!(),
            line!(),
        )
    }};
}

#[macro_export]
macro_rules! warn {
    ($message:expr) => {{
        $crate::logging::log_message(
            hyperlight_flatbuffers::flatbuffer_wrappers::guest_log_level::LogLevel::Warning,
            $message,
            module_path!(),
            // there is no way to defnitively get the caller name
            "Unknown",
            file!(),
            line!(),
        )
    }};
}

#[macro_export]
macro_rules! error {
    ($message:expr) => {{
        $crate::logging::log_message(
            hyperlight_flatbuffers::flatbuffer_wrappers::guest_log_level::LogLevel::Error,
            $message,
            module_path!(),
            // there is no way to defnitively get the caller name
            "Unknown",
            file!(),
            line!(),
        )
    }};
}

#[macro_export]
macro_rules! debug {
    ($message:expr) => {{
        $crate::logging::log_message(
            hyperlight_flatbuffers::flatbuffer_wrappers::guest_log_level::LogLevel::Debug,
            $message,
            module_path!(),
            // there is no way to defnitively get the caller name
            "Unknown",
            file!(),
            line!(),
        )
    }};
}

#[macro_export]
macro_rules! trace {
    ($message:expr) => {{
        $crate::logging::log_message(
            hyperlight_flatbuffers::flatbuffer_wrappers::guest_log_level::LogLevel::Trace,
            $message,
            module_path!(),
            // there is no way to defnitively get the caller name
            "Unknown",
            file!(),
            line!(),
        )
    }};
}
