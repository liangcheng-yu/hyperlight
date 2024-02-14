// implements  security cookie used by /GS compiler option and checks value is valid
// calls report_gsfailure if value is invalid

use crate::{__security_cookie, entrypoint::halt, guest_error::set_error};
use hyperlight_flatbuffers::flatbuffer_wrappers::guest_error::ErrorCode::GsCheckFailed;

#[no_mangle]
pub(crate) extern "C" fn __security_check_cookie(cookie: u64) {
    unsafe {
        if __security_cookie != cookie {
            set_error(GsCheckFailed, "GS Check Failed");
            // TODO: Once we fix set_error to halt() then this should be removed.
            halt();
        }
    }
}
