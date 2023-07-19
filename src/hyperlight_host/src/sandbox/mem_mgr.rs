use crate::mem::mgr::{SandboxMemoryManager, STACK_COOKIE_LEN};
use anyhow::Result;
use tracing::instrument;

pub(crate) type StackCookie = [u8; STACK_COOKIE_LEN];
pub(crate) trait MemMgr {
    /// Get an immutable reference to the internally-stored
    /// `SandboxMemoryManager`
    fn get_mem_mgr(&self) -> &SandboxMemoryManager;

    /// Get the internally-stored stack cookie that was written
    /// as a stack guard to guest memory.
    fn get_stack_cookie(&self) -> &StackCookie;

    /// Check the stack guard against the stack guard cookie stored
    /// within `self`. Return `Ok(true)` if the guard cookie could
    /// be found and it matched `self.stack_guard`, `Ok(false)` if
    /// if could be found and did not match `self.stack_guard`, and
    /// `Err` if it could not be found or there was some other error.
    #[instrument(err(Debug), skip(self))]
    fn check_stack_guard(&self) -> Result<bool> {
        self.get_mem_mgr()
            .check_stack_guard(*self.get_stack_cookie())
    }
}
