use crate::error::HyperlightError::{GuestError, OutBHandlingError, StackOverflow};
use crate::sandbox::metrics::SandboxMetric::GuestErrorCount;
use crate::MemMgrWrapper;
use crate::{int_counter_vec_inc, log_then_return, Result};
use hyperlight_flatbuffers::flatbuffer_wrappers::guest_error::{
    ErrorCode, GuestError as GuestErrorStruct,
};
/// Check for a guest error and return an `Err` if one was found,
/// and `Ok` if one was not found.
/// TODO: remove this when we hook it up to the rest of the
/// sandbox in https://github.com/deislabs/hyperlight/pull/727
pub(super) fn check_for_guest_error(mgr: &MemMgrWrapper) -> Result<()> {
    let guest_err = mgr.as_ref().get_guest_error()?;
    match guest_err.code {
        ErrorCode::NoError => Ok(()),
        ErrorCode::OutbError => match mgr.as_ref().get_host_error()? {
            Some(host_err) => {
                increment_guest_error_count(&guest_err);
                log_then_return!(OutBHandlingError(
                    host_err.source.clone(),
                    guest_err.message.clone()
                ));
            }
            // TODO: Not sure this is correct behavior. We should probably return error here but since this
            //  is a only temporary till we fix up the C APi to the Rust Sandbox its OK to leave.
            None => Ok(()),
        },
        ErrorCode::StackOverflow => {
            increment_guest_error_count(&guest_err.clone());
            log_then_return!(StackOverflow());
        }
        _ => {
            increment_guest_error_count(&guest_err.clone());
            log_then_return!(GuestError(
                guest_err.code.clone(),
                guest_err.message.clone()
            ));
        }
    }
}

fn increment_guest_error_count(guest_err: &GuestErrorStruct) {
    let guest_err_code_string: String = guest_err.code.clone().into();
    int_counter_vec_inc!(
        &GuestErrorCount,
        &[&guest_err_code_string, guest_err.message.clone().as_str()]
    );
}
