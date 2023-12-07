use std::sync::{Arc, Mutex};

use tracing::{instrument, Span};

use super::mem_mgr::MemMgrWrapper;
use crate::error::HyperlightError::StackOverflow;
use crate::hypervisor::handlers::{
    MemAccessHandler, MemAccessHandlerFunction, MemAccessHandlerWrapper,
};
use crate::{log_then_return, Result};

#[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
pub(super) fn handle_mem_access_impl(wrapper: &MemMgrWrapper) -> Result<()> {
    if !wrapper.check_stack_guard()? {
        log_then_return!(StackOverflow());
    }

    Ok(())
}

#[instrument(skip_all, parent = Span::current(), level= "Trace")]
pub(super) fn mem_access_handler_wrapper<'a>(
    wrapper: MemMgrWrapper,
) -> MemAccessHandlerWrapper<'a> {
    let mem_access_func: MemAccessHandlerFunction =
        Box::new(move || handle_mem_access_impl(&wrapper));
    let mem_access_hdl = MemAccessHandler::from(mem_access_func);
    Arc::new(Mutex::new(mem_access_hdl))
}
