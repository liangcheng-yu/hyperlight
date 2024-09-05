use alloc::vec::Vec;
use core::ffi::c_char;

use hyperlight_common::flatbuffer_wrappers::function_types::{ParameterValue, ReturnType};

use crate::host_function_call::call_host_function;

const BUFFER_SIZE: usize = 1000;

static mut MESSAGE_BUFFER: Vec<u8> = Vec::new();

/// Exposes a C API to allow the guest to print a string
///
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn _putchar(c: c_char) {
    let char = c as u8;

    // Extend buffer capacity if it's empty (like `with_capacity` in lazy_static)
    if MESSAGE_BUFFER.capacity() == 0 {
        MESSAGE_BUFFER.reserve(BUFFER_SIZE);
    }

    MESSAGE_BUFFER.push(char);

    if MESSAGE_BUFFER.len() == BUFFER_SIZE || char == b'\0' {
        // buffer is full or was passed in nullbyte, so flush
        let str = alloc::string::String::from_utf8(MESSAGE_BUFFER.clone())
            .expect("Failed to convert buffer to string");

        call_host_function(
            "HostPrint",
            Some(Vec::from(&[ParameterValue::String(str)])),
            ReturnType::Void,
        )
        .expect("Failed to call HostPrint");

        // Clear the buffer after sending
        MESSAGE_BUFFER.clear();
    }
}
