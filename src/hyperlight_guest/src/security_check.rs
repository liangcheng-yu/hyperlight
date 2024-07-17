// implements  security cookie used by /GS compiler option and checks value is valid
// calls report_gsfailure if value is invalid

use hyperlight_common::flatbuffer_wrappers::guest_error::ErrorCode::GsCheckFailed;

use crate::__security_cookie;
use crate::guest_error::set_error_and_halt;

#[no_mangle]
pub(crate) extern "C" fn __security_check_cookie(cookie: u64) {
    unsafe {
        if __security_cookie != cookie {
            set_error_and_halt(GsCheckFailed, "GS Check Failed");
        }
    }
}
