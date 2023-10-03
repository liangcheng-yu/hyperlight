use crate::{flatbuffers::hyperlight::generated::ErrorCode, MemMgrWrapper};
use anyhow::{bail, Result};
use tracing::error;

/// Check for a guest error and return an `Err` if one was found,
/// and `Ok` if one was not found.
/// TODO: remove this when we hook it up to the rest of the
/// sandbox in https://github.com/deislabs/hyperlight/pull/727
pub fn check_for_guest_error(mgr: &MemMgrWrapper) -> Result<()> {
    let guest_err = mgr.as_ref().get_guest_error()?;
    match guest_err.code {
        ErrorCode::NoError => Ok(()),
        ErrorCode::OutbError => match mgr.as_ref().get_host_error()? {
            Some(host_err) => bail!("[OutB Error] {:?}: {:?}", guest_err.code, host_err),
            None => Ok(()),
        },
        ErrorCode::StackOverflow => {
            let err_msg = format!(
                "[Stack Overflow] Guest Error: {:?}: {}",
                guest_err.code, guest_err.message
            );
            error!("{}", err_msg);
            bail!(err_msg);
        }
        _ => {
            let err_msg = format!("Guest Error: {:?}: {}", guest_err.code, guest_err.message);
            error!("{}", err_msg);
            bail!(err_msg);
        }
    }
}
