use core::{ffi::c_void, slice::from_raw_parts};

use hyperlight_common::flatbuffer_wrappers::guest_error::{ErrorCode, GuestError};

use crate::P_PEB;

pub(crate) fn check_for_host_error() {
    unsafe {
        let peb_ptr = P_PEB.unwrap();
        let guest_error_buffer_ptr = (*peb_ptr).guestErrorData.guestErrorBuffer as *mut u8;
        let guest_error_buffer_size = (*peb_ptr).guestErrorData.guestErrorSize as usize;

        let guest_error_buffer = from_raw_parts(guest_error_buffer_ptr, guest_error_buffer_size);

        if !guest_error_buffer.is_empty() {
            let guest_error = GuestError::try_from(guest_error_buffer).expect("Invalid GuestError");
            if guest_error.code != ErrorCode::NoError {
                (*peb_ptr).outputdata.outputDataBuffer = usize::MAX as *mut c_void;
                panic!(
                    "Guest Error: {:?} - {}",
                    guest_error.code, guest_error.message
                );
            }
        }
    }
}
