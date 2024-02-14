use core::ffi::c_char;
use core::ffi::c_void;
use core::ffi::CStr;

use alloc::{string::ToString, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::guest_error::{ErrorCode, GuestError};

use crate::{entrypoint::halt, P_PEB};

pub(crate) fn write_error(error_code: ErrorCode, message: Option<&str>) {
    let guest_error = GuestError::new(
        error_code,
        message.map_or("".to_string(), |m| m.to_string()),
    );
    let guest_error_buffer: Vec<u8> = (&guest_error).try_into().unwrap();

    unsafe {
        // TODO: We should probably review the use of asserts and do explicit checks here
        // these asserts will cause a panic and we wont get much in the way of error information

        assert!(!(*P_PEB.unwrap()).pGuestErrorBuffer.is_null());
        assert!(guest_error_buffer.len() <= (*P_PEB.unwrap()).guestErrorBufferSize as usize);

        // Optimally, we'd use copy_from_slice here, but, because
        // p_guest_error_buffer is a *mut c_void, we can't do that.
        // Instead, we do the copying manually using pointer arithmetic.
        // Plus; before, we'd do an assert w/ the result from copy_from_slice,
        // but, because copy_nonoverlapping doesn't return anything, we can't do that.
        // Instead, we do the prior asserts to check the destination pointer isn't null
        // and that there is enough space in the destination buffer for the copy.
        let dest_ptr = (*P_PEB.unwrap()).pGuestErrorBuffer as *mut u8;
        core::ptr::copy_nonoverlapping(
            guest_error_buffer.as_ptr(),
            dest_ptr,
            guest_error_buffer.len(),
        );
    }
}

pub(crate) fn reset_error() {
    unsafe {
        let peb_ptr = P_PEB.unwrap();
        core::ptr::write_bytes(
            (*peb_ptr).pGuestErrorBuffer,
            0,
            (*peb_ptr).guestErrorBufferSize as usize,
        );
    }
}

pub fn set_error(error_code: ErrorCode, message: &str) {
    write_error(error_code, Some(message));
    unsafe {
        (*P_PEB.unwrap()).outputdata.outputDataBuffer = usize::MAX as *mut c_void;
    }
}

// Exposes a C API to allow the guest to set an error

#[no_mangle]
#[allow(non_camel_case_types)]
pub extern "C" fn setError(code: u64, message: *const c_char) {
    let error_code = ErrorCode::from(code);
    match message.is_null() {
        true => write_error(error_code, None),
        false => {
            let message = match unsafe { CStr::from_ptr(message).to_str().ok() } {
                Some(s) => s,
                None => "Invalid error message, could not be converted to a string",
            };
            write_error(error_code, Some(message));
        }
    }
    halt();
}
