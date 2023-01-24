use super::config::SandboxMemoryConfiguration;
use super::guest_mem::GuestMemory;
use super::layout::SandboxMemoryLayout;
use anyhow::Result;
use std::cmp::Ordering;

/// Whether or not the 64-bit page directory entry (PDE) record is
/// present.
///
/// See the following links explaining a PDE in various levels of detail:
///
/// * Very basic description: https://stackoverflow.com/a/26945892
/// * More in-depth descriptions: https://wiki.osdev.org/Paging
const PDE64_PRESENT: u64 = 1;
/// Read/write permissions flag for the 64-bit PDE
const PDE64_RW: u64 = 1 << 1;
/// The user/supervisor bit for the 64-bit PDE
const PDE64_USER: u64 = 1 << 2;
/// The page size for the 64-bit PDE
const PDE64_PS: u64 = 1 << 7;

/// A struct that is responsible for laying out and managing the memory
/// for a given `Sandbox`.
pub struct SandboxMemoryManager {
    _cfg: SandboxMemoryConfiguration,
    run_from_process_memory: bool,
}

impl SandboxMemoryManager {
    /// Create a new `SandboxMemoryManager` with the given parameters
    pub fn new(_cfg: SandboxMemoryConfiguration, run_from_process_memory: bool) -> Self {
        Self {
            _cfg,
            run_from_process_memory,
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

    /// Set up the hypervisor partition in the given `GuestMemory` parameter
    /// `guest_mem`, with the given memory size `mem_size`
    pub fn set_up_hypervisor_partition(
        &self,
        guest_mem: &mut GuestMemory,
        mem_size: u64,
    ) -> Result<u64> {
        // Add 0x200000 because that's the start of mapped memory
        // For MSVC, move rsp down by 0x28.  This gives the called 'main'
        // function the appearance that rsp was was 16 byte aligned before
        //the 'call' that calls main (note we don't really have a return value
        // on the stack but some assembly instructions are expecting rsp have
        // started 0x8 bytes off of 16 byte alignment when 'main' is invoked.
        // We do 0x28 instead of 0x8 because MSVC can expect that there are
        // 0x20 bytes of space to write to by the called function.
        // I am not sure if this happens with the 'main' method, but we do this
        // just in case.
        // NOTE: We do this also for GCC freestanding binaries because we
        // specify __attribute__((ms_abi)) on the start method
        let rsp = mem_size + SandboxMemoryLayout::BASE_ADDRESS as u64 - 0x28;

        // Create pagetable
        guest_mem.write_u64(
            SandboxMemoryLayout::PML4_OFFSET,
            PDE64_PRESENT | PDE64_RW | PDE64_USER | SandboxMemoryLayout::PDPT_GUEST_ADDRESS as u64,
        )?;
        guest_mem.write_u64(
            SandboxMemoryLayout::PDPT_OFFSET,
            PDE64_PRESENT | PDE64_RW | PDE64_USER | SandboxMemoryLayout::PD_GUEST_ADDRESS as u64,
        )?;

        // do not map first 2 megs
        for i in 0..512 {
            let offset: usize = SandboxMemoryLayout::PD_OFFSET + (i * 8);
            // map each VA to physical memory 2 megs lower
            let val_to_write: u64 =
                (i << 21) as u64 + (PDE64_PRESENT | PDE64_RW | PDE64_USER | PDE64_PS);
            guest_mem.write_u64(offset, val_to_write)?;
        }
        Ok(rsp)
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

    /// Get the process environment block (PEB) address assuming `start_addr`
    /// is the address of the start of memory, using the given
    /// `SandboxMemoryLayout` to calculate the address.
    ///
    /// For more details on PEBs, please see the following link:
    ///
    /// https://en.wikipedia.org/wiki/Process_Environment_Block
    pub fn get_peb_address(&self, layout: &SandboxMemoryLayout, start_addr: u64) -> u64 {
        match self.run_from_process_memory {
            true => layout.get_in_process_peb_offset() as u64 + start_addr,
            false => layout.peb_address as u64,
        }
    }
}
