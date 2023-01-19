use super::config::SandboxMemoryConfiguration;
use super::guest_mem::GuestMemory;
use super::layout::SandboxMemoryLayout;
use anyhow::Result;

/// A struct that is responsible for laying out and managing the memory
/// for a given `Sandbox`.
pub struct SandboxMemoryManager {
    _cfg: SandboxMemoryConfiguration,
    _run_from_process_memory: bool,
}

impl SandboxMemoryManager {
    /// Create a new `SandboxMemoryManager` with the given parameters
    pub fn new(_cfg: SandboxMemoryConfiguration, _run_from_process_memory: bool) -> Self {
        Self {
            _cfg,
            _run_from_process_memory,
        }
    }

    /// Set the stack guard to `cookie` using `layout` to calculate
    /// its location and `guest_mem` to write it.
    pub fn set_stack_guard(
        &self,
        layout: &SandboxMemoryLayout,
        guest_mem: &mut GuestMemory,
        cookie: &Vec<u8>,
    ) -> Result<()> {
        let stack_offset = layout.get_top_of_stack_offset();
        guest_mem.copy_from_slice(cookie.as_slice(), stack_offset)
    }
}
