use std::sync::{Arc, Mutex};

use super::mem_mgr::MemMgrWrapper;
use crate::hypervisor::handlers::{
    MemAccessHandler, MemAccessHandlerFunction, MemAccessHandlerWrapper,
};
use anyhow::{bail, Result};

pub(super) fn handle_mem_access_impl(wrapper: &MemMgrWrapper) -> Result<()> {
    if !wrapper.check_stack_guard()? {
        bail!("Stack overflow detected");
    }

    Ok(())
}

pub(super) fn mem_access_handler_wrapper<'a>(
    wrapper: MemMgrWrapper,
) -> MemAccessHandlerWrapper<'a> {
    let mem_access_func: MemAccessHandlerFunction =
        Box::new(move || handle_mem_access_impl(&wrapper));
    let mem_access_hdl = MemAccessHandler::from(mem_access_func);
    Arc::new(Mutex::new(mem_access_hdl))
}
