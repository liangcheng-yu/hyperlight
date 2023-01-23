use super::config::SandboxMemoryConfiguration;
use super::guest_mem::GuestMemory;
use super::layout::SandboxMemoryLayout;
use anyhow::Result;
use std::cmp::Ordering;

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
    ///
    /// Currently, this method could be an associated function but is
    /// still a method because I (arschles) want to make this `struct` hold a
    /// reference to a `SandboxMemoryLayout` and `GuestMemory`,
    /// remove the `layout` and `guest_mem` parameters, and use
    /// the `&self` to access them instead.
    pub fn set_stack_guard(
        &self,
        layout: &SandboxMemoryLayout,
        guest_mem: &mut GuestMemory,
        cookie: &Vec<u8>,
    ) -> Result<()> {
        let stack_offset = layout.get_top_of_stack_offset();
        guest_mem.copy_from_slice(cookie.as_slice(), stack_offset)
    }

    /// Check the stack guard of the memory in `guest_mem`, using
    /// `layout` to calculate its location.
    ///
    /// Return `true`
    /// if `guest_mem` could be accessed properly and the guard
    /// matches `cookie`. If it could be accessed properly and the
    /// guard doesn't match `cookie`, return `false`. Otherwise, return
    /// a descriptive error.
    ///
    /// This method could be an associated function instead. See
    /// documentation at the bottom `set_stack_guard` for description
    /// of why it isn't.
    pub fn check_stack_guard(
        &self,
        layout: &SandboxMemoryLayout,
        guest_mem: &GuestMemory,
        cookie: &Vec<u8>,
    ) -> Result<bool> {
        let offset = layout.get_top_of_stack_offset();
        let mut test_cookie = vec![b'\0'; cookie.len()];
        guest_mem.copy_to_slice(test_cookie.as_mut_slice(), offset)?;

        let cmp_res = cookie.iter().cmp(test_cookie.iter());
        Ok(cmp_res == Ordering::Equal)
    }
}
