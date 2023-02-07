use super::guest_mem_snapshot::GuestMemorySnapshot;
use super::layout::SandboxMemoryLayout;
use super::ptr::{GuestPtr, HostPtr};
use super::{config::SandboxMemoryConfiguration, ptr_addr_space::HostAddressSpace};
use super::{guest_mem::GuestMemory, ptr_addr_space::GuestAddressSpace};
use anyhow::{bail, Result};
use core::mem::size_of;
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
    /// Whether or not to run a sandbox in-process
    pub run_from_process_memory: bool,
    mem_snapshot: Option<GuestMemorySnapshot>,
}

impl SandboxMemoryManager {
    /// Create a new `SandboxMemoryManager` with the given parameters
    pub fn new(_cfg: SandboxMemoryConfiguration, run_from_process_memory: bool) -> Self {
        Self {
            _cfg,
            run_from_process_memory,
            mem_snapshot: None,
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

    /// Create a new memory snapshot of the given `GuestMemory` and
    /// store it internally. Return an `Ok(())` if the snapshot
    /// operation succeeded, and an `Err` otherwise.
    pub fn snapshot_state(&mut self, guest_mem: &GuestMemory) -> Result<()> {
        let snap = &mut self.mem_snapshot;
        if let Some(snapshot) = snap {
            snapshot.replace_snapshot()
        } else {
            let new_snapshot = GuestMemorySnapshot::new(guest_mem.clone())?;
            self.mem_snapshot = Some(new_snapshot);
            Ok(())
        }
    }

    /// Restore memory from the pre-existing snapshot and return
    /// `Ok(())`. Return an `Err` if there was no pre-existing
    /// snapshot, or there was but there was an error restoring.
    pub fn restore_state(&mut self) -> Result<()> {
        let snap = &mut self.mem_snapshot;
        if let Some(snapshot) = snap {
            snapshot.restore_from_snapshot()
        } else {
            bail!("restore_state called with no valid snapshot");
        }
    }

    /// Get the return value of an executable that ran, or an `Err`
    /// if no such return value was present.
    pub fn get_return_value(
        &self,
        guest_mem: &GuestMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<i32> {
        let offset = layout.output_data_buffer_offset;
        guest_mem.read_i32(offset as u64)
    }

    /// Sets `addr` to the correct offset in the memory referenced by
    /// `guest_mem` to indicate the address of the outb pointer
    pub fn set_outb_address(
        &self,
        guest_mem: &mut GuestMemory,
        layout: &SandboxMemoryLayout,
        addr: u64,
    ) -> Result<()> {
        let offset = layout.get_out_b_pointer_offset();
        guest_mem.write_u64(offset, addr)
    }

    /// Get the offset to use when calculating addresses
    pub fn get_address_offset(&self, source_addr: u64) -> u64 {
        match self.run_from_process_memory {
            true => 0,
            false => source_addr - SandboxMemoryLayout::BASE_ADDRESS as u64,
        }
    }

    /// Convert a pointer in the guest's address space to a pointer in the
    /// host's.
    pub fn get_host_address_from_ptr(
        &self,
        guest_ptr: GuestPtr,
        shared_mem: &GuestMemory,
    ) -> Result<HostPtr> {
        // to translate a pointer from the guest address space,
        // we need to get the offset (which is already taken care of in
        // guest_ptr) and then add it to the host base address, which is
        // the base address of shared memory.
        guest_ptr.to_foreign_ptr(HostAddressSpace::new(
            shared_mem,
            self.run_from_process_memory,
        ))
    }

    /// Convert a pointer in the host's address space to a pointer in the
    /// guest's.
    pub fn get_guest_address_from_ptr(&self, host_ptr: HostPtr) -> Result<GuestPtr> {
        // to convert a pointer in the host address space, we need to get its
        // offset (which is already done inside host_ptr) and then
        // add it to the base address inside guest memory, which is
        // below.
        host_ptr.to_foreign_ptr(GuestAddressSpace::new(self.run_from_process_memory))
    }

    /// Get the address of the dispatch function in memory
    pub fn get_pointer_to_dispatch_function(
        &self,
        guest_mem: &GuestMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<u64> {
        guest_mem.read_u64(layout.get_dispatch_function_pointer_offset() as u64)
    }

    /// Get output from the guest as a `String`
    pub fn get_string_output(
        &self,
        layout: &SandboxMemoryLayout,
        guest_mem: &GuestMemory,
    ) -> Result<String> {
        let offset = layout.get_output_data_offset();
        guest_mem.read_string(offset)
    }

    /// Get the length of the host exception
    pub fn get_host_exception_length(
        &self,
        layout: &SandboxMemoryLayout,
        guest_mem: &GuestMemory,
    ) -> Result<i32> {
        let offset = layout.get_host_exception_offset() as u64;
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        guest_mem.read_i32(offset)
    }

    /// Get a bool indicating if there is a host exception
    pub fn has_host_exception(
        &self,
        layout: &SandboxMemoryLayout,
        guest_mem: &GuestMemory,
    ) -> Result<bool> {
        let offset = layout.get_host_exception_offset() as u64;
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        let len = guest_mem.read_i32(offset)?;
        Ok(len != 0)
    }

    /// Get the exception data that was written by the Hyperlight Host
    /// Returns a `Result` containing 'Unit' or an error.
    /// Writes the exception data to the buffer at `exception_data_ptr`.
    pub fn get_host_exception_data(
        &self,
        layout: &SandboxMemoryLayout,
        guest_mem: &GuestMemory,
        exception_data_slc: &mut [u8],
    ) -> Result<()> {
        let offset = layout.get_host_exception_offset();
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        let len = guest_mem.read_i32(offset as u64)?;
        let exception_data_slc_len = exception_data_slc.len();
        if exception_data_slc_len != len as usize {
            bail!(
                "Exception data length mismatch. Got {}, expected {}",
                exception_data_slc_len,
                len
            );
        }
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        guest_mem.copy_to_slice(exception_data_slc, offset + size_of::<i32>())?;
        Ok(())
    }

    /// This function writes an exception to guest memory and is intended to be used when an exception occurs
    /// handling an outb call from the guest
    pub fn write_outb_exception(
        &self,
        layout: &SandboxMemoryLayout,
        guest_mem: &mut GuestMemory,
        guest_error_msg: &Vec<u8>,
        host_exception_data: &Vec<u8>,
    ) -> Result<()> {
        const OUTB_ERROR: u64 = 7;
        let err_code_offset = layout.guest_error_offset;

        // write the error code to memory
        guest_mem.write_u64(err_code_offset, OUTB_ERROR)?;

        let err_code_size_offset = layout.get_guest_error_message_size_offset() as u64;
        let max_err_msg_size = guest_mem.read_u64(err_code_size_offset)?;
        if guest_error_msg.len() as u64 > max_err_msg_size {
            bail!(
                "Guest error message is too large. Max size is {} Got {}",
                max_err_msg_size,
                guest_error_msg.len()
            );
        }

        let err_msg_offset = layout.guest_error_message_buffer_offset;
        guest_mem.copy_from_slice(guest_error_msg, err_msg_offset)?;

        let host_exception_offset = layout.get_host_exception_offset();
        let host_exception_size_offset = layout.get_host_exception_size_offset();
        let max_host_exception_size =
            guest_mem.read_u64(host_exception_size_offset as u64)? as usize;

        // First four bytes of host exception are length

        if host_exception_data.len() > max_host_exception_size - size_of::<i32>() {
            bail!(
                "Host exception message is too large. Max size is {} Got {}",
                max_host_exception_size,
                host_exception_data.len()
            );
        }

        guest_mem.write_i32(host_exception_offset, host_exception_data.len() as i32)?;
        guest_mem.copy_from_slice(
            host_exception_data,
            host_exception_offset + size_of::<i32>(),
        )?;

        Ok(())
    }
}
