use crate::mem::{
    layout::SandboxMemoryLayout,
    mgr::{SandboxMemoryManager, STACK_COOKIE_LEN},
};
use anyhow::Result;
use tracing::instrument;

pub type StackCookie = [u8; STACK_COOKIE_LEN];

pub trait MemMgrWrapperGetter {
    fn get_mem_mgr_wrapper(&self) -> &MemMgrWrapper;
    fn get_mem_mgr_wrapper_mut(&mut self) -> &mut MemMgrWrapper;
}

#[derive(Clone)]
pub struct MemMgrWrapper(SandboxMemoryManager, StackCookie);

impl MemMgrWrapper {
    pub(super) fn new(mgr: SandboxMemoryManager, stack_cookie: StackCookie) -> Self {
        Self(mgr, stack_cookie)
    }

    fn get_mgr(&self) -> &SandboxMemoryManager {
        &self.0
    }

    fn get_mgr_mut(&mut self) -> &mut SandboxMemoryManager {
        &mut self.0
    }
    pub(super) fn get_stack_cookie(&self) -> &StackCookie {
        &self.1
    }

    /// Check the stack guard against the given `stack_cookie`.
    ///
    /// Return `Ok(true)` if the given cookie matches the one in guest memory,
    /// and `Ok(false)` otherwise. Return `Err` if it could not be found or
    /// there was some other error.
    #[instrument(err(Debug), skip(self))]
    pub(crate) fn check_stack_guard(&self) -> Result<bool> {
        self.get_mgr().check_stack_guard(*self.get_stack_cookie())
    }

    pub(super) fn write_memory_layout(&mut self, run_from_process_memory: bool) -> Result<()> {
        let mgr = self.get_mgr_mut();
        let layout = mgr.layout;
        let shared_mem = mgr.get_shared_mem_mut();
        let mem_size = shared_mem.mem_size();
        let guest_offset = if run_from_process_memory {
            shared_mem.base_addr()
        } else {
            SandboxMemoryLayout::BASE_ADDRESS
        };
        layout.write(shared_mem, guest_offset, mem_size)
    }
}

impl AsMut<SandboxMemoryManager> for MemMgrWrapper {
    fn as_mut(&mut self) -> &mut SandboxMemoryManager {
        self.get_mgr_mut()
    }
}

impl AsRef<SandboxMemoryManager> for MemMgrWrapper {
    fn as_ref(&self) -> &SandboxMemoryManager {
        self.get_mgr()
    }
}
