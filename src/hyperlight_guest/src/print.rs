use core::ffi::c_char;

use alloc::vec::Vec;
use hyperlight_flatbuffers::flatbuffer_wrappers::function_types::{ParameterValue, ReturnType};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::host_function_call::call_host_function;

const BUFFER_SIZE: usize = 1000;

lazy_static! {
    static ref MESSAGE_LOCK: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(BUFFER_SIZE));
}

/// Exposes a C API to allow the guest to print a string
///
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn _putchar(c: c_char) {
    let char = c as u8;
    let mut buffer = MESSAGE_LOCK.lock();
    buffer.push(char);

    if buffer.len() == BUFFER_SIZE || char == b'\0' {
        // buffer is full or was passed in nullbyte, so flush
        let str = alloc::string::String::from_utf8(buffer.to_vec())
            .expect("Failed to convert buffer to string");
        call_host_function(
            "HostPrint",
            Some(Vec::from(&[ParameterValue::String(str)])),
            ReturnType::Void,
        )
        .expect("Failed to call HostPrint"); // optimally, this would be a `?`, but we need to match the expected `extern` def
        buffer.clear();
    }
}
