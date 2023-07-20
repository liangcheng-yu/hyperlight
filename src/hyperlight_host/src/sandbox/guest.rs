use crate::func::{guest::GuestFunction, ret_type::SupportedReturnType, types::ReturnValue};
use anyhow::Result;

use super::mem_mgr::MemMgr;

/// Enables the host to call functions in the guest and have the sandbox state reset at the start of the call
pub(crate) trait CallGuestFunction<'a>: MemMgr {
    fn call_guest_function<T, R>(&self, function: T) -> Result<ReturnValue>
    where
        T: GuestFunction<'a, R>,
        R: SupportedReturnType<R>,
    {
        // TODO: call reset_state() here

        function.call() // <- ensures that only one call can be made concurrently
    }
}
