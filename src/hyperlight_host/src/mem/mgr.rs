use super::config::SandboxMemoryConfiguration;
use super::ptr::{GuestPtr, HostPtr};
use super::ptr_addr_space::HostAddressSpace;
use super::ptr_offset::Offset;
use super::shared_mem_snapshot::SharedMemorySnapshot;
use super::{layout::SandboxMemoryLayout, pe::pe_info::PEInfo};
use super::{pe::headers::PEHeaders, ptr::RawPtr};
use super::{ptr_addr_space::GuestAddressSpace, shared_mem::SharedMemory};
use crate::{
    capi::arrays::raw_vec::RawVec,
    guest::guest_error::{Code, GuestError},
    guest::guest_function_call::GuestFunctionCall,
};
use anyhow::{anyhow, bail, Result};
use core::mem::size_of;
use readonly;
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
#[readonly::make]
pub struct SandboxMemoryManager {
    pub mem_cfg: SandboxMemoryConfiguration,
    /// Whether or not to run a sandbox in-process
    pub run_from_process_memory: bool,
    mem_snapshot: Option<SharedMemorySnapshot>,
    pub shared_mem: SharedMemory,
    pub layout: SandboxMemoryLayout,
    pub load_addr: RawPtr,
    pub entrypoint_offset: Offset,
}

impl SandboxMemoryManager {
    /// Create a new `SandboxMemoryManager` with the given parameters
    pub fn new(
        mem_cfg: SandboxMemoryConfiguration,
        layout: SandboxMemoryLayout,
        shared_mem: SharedMemory,
        run_from_process_memory: bool,
        load_addr: RawPtr,
        entrypoint_offset: Offset,
    ) -> Self {
        Self {
            mem_cfg,
            run_from_process_memory,
            mem_snapshot: None,
            layout,
            shared_mem,
            load_addr,
            entrypoint_offset,
        }
    }

    /// Get mutable `SharedMemory` associated with `self`
    fn get_shared_mem_mut(&mut self) -> &mut SharedMemory {
        &mut self.shared_mem
    }

    /// Set the stack guard to `cookie` using `layout` to calculate
    /// its location and `shared_mem` to write it.
    ///
    /// Currently, this method could be an associated function but is
    /// still a method because I (arschles) want to make this `struct` hold a
    /// reference to a `SandboxMemoryLayout` and `SharedMemory`,
    /// remove the `layout` and `shared_mem` parameters, and use
    /// the `&self` to access them instead.
    pub fn set_stack_guard(&mut self, cookie: &Vec<u8>) -> Result<()> {
        let stack_offset = self.layout.get_top_of_stack_offset();
        self.shared_mem
            .copy_from_slice(cookie.as_slice(), stack_offset)
    }

    /// Set up the hypervisor partition in the given `SharedMemory` parameter
    /// `shared_mem`, with the given memory size `mem_size`
    pub fn set_up_hypervisor_partition(&mut self, mem_size: u64) -> Result<u64> {
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
        self.shared_mem.write_u64(
            Offset::try_from(SandboxMemoryLayout::PML4_OFFSET)?,
            PDE64_PRESENT | PDE64_RW | PDE64_USER | SandboxMemoryLayout::PDPT_GUEST_ADDRESS as u64,
        )?;
        self.shared_mem.write_u64(
            Offset::try_from(SandboxMemoryLayout::PDPT_OFFSET)?,
            PDE64_PRESENT | PDE64_RW | PDE64_USER | SandboxMemoryLayout::PD_GUEST_ADDRESS as u64,
        )?;

        // do not map first 2 megs
        for i in 0..512 {
            let offset = Offset::try_from(SandboxMemoryLayout::PD_OFFSET + (i * 8))?;
            // map each VA to physical memory 2 megs lower
            let val_to_write: u64 =
                (i << 21) as u64 + (PDE64_PRESENT | PDE64_RW | PDE64_USER | PDE64_PS);
            self.shared_mem.write_u64(offset, val_to_write)?;
        }
        Ok(rsp)
    }

    /// Check the stack guard of the memory in `shared_mem`, using
    /// `layout` to calculate its location.
    ///
    /// Return `true`
    /// if `shared_mem` could be accessed properly and the guard
    /// matches `cookie`. If it could be accessed properly and the
    /// guard doesn't match `cookie`, return `false`. Otherwise, return
    /// a descriptive error.
    ///
    /// This method could be an associated function instead. See
    /// documentation at the bottom `set_stack_guard` for description
    /// of why it isn't.
    pub fn check_stack_guard(&self, cookie: &Vec<u8>) -> Result<bool> {
        let offset = self.layout.get_top_of_stack_offset();
        let mut test_cookie = vec![b'\0'; cookie.len()];
        self.shared_mem
            .copy_to_slice(test_cookie.as_mut_slice(), offset)?;

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
    pub fn get_peb_address(&self, start_addr: u64) -> Result<u64> {
        match self.run_from_process_memory {
            true => {
                let updated_offset = self.layout.get_in_process_peb_offset() + start_addr;
                Ok(u64::from(updated_offset))
            }
            false => u64::try_from(self.layout.peb_address).map_err(|_| {
                anyhow!(
                    "get_peb_address: failed to convert peb_address ({}) to u64",
                    self.layout.peb_address
                )
            }),
        }
    }

    /// Create a new memory snapshot of the given `SharedMemory` and
    /// store it internally. Return an `Ok(())` if the snapshot
    /// operation succeeded, and an `Err` otherwise.
    pub fn snapshot_state(&mut self) -> Result<()> {
        let snap = &mut self.mem_snapshot;
        if let Some(snapshot) = snap {
            snapshot.replace_snapshot()
        } else {
            let new_snapshot = SharedMemorySnapshot::new(self.shared_mem.clone())?;
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
    pub fn get_return_value(&self) -> Result<i32> {
        let offset = self.layout.output_data_buffer_offset;
        self.shared_mem.read_i32(offset)
    }

    /// Sets `addr` to the correct offset in the memory referenced by
    /// `shared_mem` to indicate the address of the outb pointer
    pub fn set_outb_address(&mut self, addr: u64) -> Result<()> {
        let offset = self.layout.get_out_b_pointer_offset();
        self.shared_mem.write_u64(offset, addr)
    }

    /// Get the name of the method called by the host
    pub fn get_host_call_method_name(&self) -> Result<String> {
        let output_data_offset = self.layout.output_data_buffer_offset;
        let str_addr = self.shared_mem.read_u64(output_data_offset)?;
        if self.run_from_process_memory {
            // if we're running in-process, the string method name starts directly
            // at the absolute in-memory address str_addr
            let str_len = unsafe { libc::strlen(str_addr as *const i8) };
            let str_raw_vec = unsafe { RawVec::copy_from_ptr(str_addr as *mut u8, str_len) };
            String::from_utf8(str_raw_vec.into()).map_err(|e| anyhow!(e))
        } else {
            // if we're running in the VM, the string method name starts at offset
            // str_addr - BASE_ADDRESS. We can calculate that offset using GuestPtr
            let str_guest_ptr = GuestPtr::try_from(RawPtr::from(str_addr))?;
            self.shared_mem.read_string(str_guest_ptr.offset())
        }
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
    pub fn get_host_address_from_ptr(&self, guest_ptr: GuestPtr) -> Result<HostPtr> {
        // to translate a pointer from the guest address space,
        // we need to get the offset (which is already taken care of in
        // guest_ptr) and then add it to the host base address, which is
        // the base address of shared memory.
        guest_ptr.to_foreign_ptr(HostAddressSpace::new(&self.shared_mem)?)
    }

    /// Convert a pointer in the host's address space to a pointer in the
    /// guest's.
    pub fn get_guest_address_from_ptr(&self, host_ptr: HostPtr) -> Result<GuestPtr> {
        // to convert a pointer in the host address space, we need to get its
        // offset (which is already done inside host_ptr) and then
        // add it to the base address inside guest memory, which is
        // below.
        host_ptr.to_foreign_ptr(GuestAddressSpace::new()?)
    }

    /// Get the address of the dispatch function in memory
    pub fn get_pointer_to_dispatch_function(&self) -> Result<u64> {
        self.shared_mem
            .read_u64(self.layout.get_dispatch_function_pointer_offset())
    }

    /// Get output from the guest as a `String`
    pub fn get_string_output(&self) -> Result<String> {
        let offset = self.layout.get_output_data_offset();
        self.shared_mem.read_string(offset)
    }

    /// Get the length of the host exception
    pub fn get_host_exception_length(&self) -> Result<i32> {
        let offset = self.layout.get_host_exception_offset();
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        self.shared_mem.read_i32(offset)
    }

    /// Get a bool indicating if there is a host exception
    pub fn has_host_exception(&self) -> Result<bool> {
        let offset = self.layout.get_host_exception_offset();
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        let len = self.shared_mem.read_i32(offset)?;
        Ok(len != 0)
    }

    /// Get the exception data that was written by the Hyperlight Host
    /// Returns a `Result` containing 'Unit' or an error.
    /// Writes the exception data to the buffer at `exception_data_ptr`.
    pub fn get_host_exception_data(&self, exception_data_slc: &mut [u8]) -> Result<()> {
        let offset = self.layout.get_host_exception_offset();
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        let len = self.shared_mem.read_i32(offset)?;
        let exception_data_slc_len = exception_data_slc.len();
        if exception_data_slc_len != len as usize {
            bail!(
                "Exception data length mismatch. Got {}, expected {}",
                exception_data_slc_len,
                len
            );
        }
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        self.shared_mem
            .copy_to_slice(exception_data_slc, offset + size_of::<i32>())?;
        Ok(())
    }

    /// This function writes an exception to guest memory and is intended to be used when an exception occurs
    /// handling an outb call from the guest
    pub fn write_outb_exception(
        &mut self,
        guest_error_msg: &Vec<u8>,
        host_exception_data: &Vec<u8>,
    ) -> Result<()> {
        let message = String::from_utf8(guest_error_msg.to_owned()).map_err(|e| anyhow!(e))?;
        let ge = GuestError::new(Code::OutbError, message);
        ge.write_to_memory(&mut self.shared_mem, &self.layout)?;

        let host_exception_offset = self.layout.get_host_exception_offset();
        let host_exception_size_offset = self.layout.get_host_exception_size_offset();
        let max_host_exception_size = {
            let size_u64 = self.shared_mem.read_u64(host_exception_size_offset)?;
            usize::try_from(size_u64)
        }?;

        // First four bytes of host exception are length

        if host_exception_data.len() > max_host_exception_size - size_of::<i32>() {
            bail!(
                "Host exception message is too large. Max size is {} Got {}",
                max_host_exception_size,
                host_exception_data.len()
            );
        }

        self.shared_mem
            .write_i32(host_exception_offset, host_exception_data.len() as i32)?;
        self.shared_mem.copy_from_slice(
            host_exception_data,
            host_exception_offset + size_of::<i32>(),
        )?;

        Ok(())
    }

    /// Get the guest error data
    pub fn get_guest_error(&self) -> Result<GuestError> {
        GuestError::try_from((&self.shared_mem, &self.layout))
    }

    /// Load the binary represented by `pe_info` into memory, ensuring
    /// all necessary relocations are made prior to completing the load
    /// operation, then create a new `SharedMemory` to store the new PE
    /// file and a `SandboxMemoryLayout` to describe the layout of that
    /// new `SharedMemory`.
    ///
    /// Returns the following:
    ///
    /// - The newly-created `SharedMemory`
    /// - The `SandboxMemoryLayout` describing that `SharedMemory`
    /// - The offset to the entrypoint. This value means something different
    /// depending on whether we're using in-process mode or not:
    ///     - If we're using in-process mode, this value will be into
    ///     host memory
    ///     - If we're not running with in-memory mode, this value will be
    ///     into guest memory
    pub fn load_guest_binary_into_memory(
        cfg: SandboxMemoryConfiguration,
        pe_info: &mut PEInfo,
        run_from_process_memory: bool,
    ) -> Result<Self> {
        let layout = SandboxMemoryLayout::new(
            cfg,
            pe_info.get_payload_len(),
            usize::try_from(pe_info.stack_reserve())?,
            usize::try_from(pe_info.heap_reserve())?,
        )?;
        let mem_size = layout.get_memory_size()?;
        let mut shared_mem = SharedMemory::new(mem_size)?;

        let load_addr = if run_from_process_memory {
            // if we're running in-process, load_addr is the absolute
            // address to the start of shared memory, plus the offset to
            // code
            shared_mem.base_addr() + SandboxMemoryLayout::CODE_OFFSET
        } else {
            // otherwise, we're running in a VM, so load_addr
            // is the base address in a VM plus the code
            // offset
            SandboxMemoryLayout::GUEST_CODE_ADDRESS
        };

        let entrypoint_offset = Offset::from({
            // we have to create this intermediate variable to ensure
            // we have an _immutable_ reference to a `PEInfo`, which
            // is what the PEHeaders::from expects
            let pe_info_immut: &PEInfo = pe_info;
            let pe_headers = PEHeaders::from(pe_info_immut);
            pe_headers.entrypoint_offset
        });

        let relocation_patches =
            pe_info.get_exe_relocation_patches(pe_info.get_payload(), load_addr)?;

        {
            // Apply relocations to the PE file (if necessary), then copy
            // the PE file into shared memory
            PEInfo::apply_relocation_patches(pe_info.get_payload_mut(), relocation_patches)?;
            let code_offset = Offset::try_from(SandboxMemoryLayout::CODE_OFFSET)?;
            shared_mem.copy_from_slice(pe_info.get_payload(), code_offset)
        }?;

        // Write a pointer to code so that guest exe can check it's running
        // in Hyperlight
        {
            let offset = layout.get_code_pointer_offset();
            let load_addr_u64: u64 = load_addr.try_into()?;
            shared_mem.write_u64(offset, load_addr_u64)?;
        }
        Ok(Self::new(
            cfg,
            layout,
            shared_mem,
            run_from_process_memory,
            RawPtr::try_from(load_addr)?,
            entrypoint_offset,
        ))
    }

    /// Writes a guest function call to memory
    pub fn write_guest_function_call(&mut self, buffer: &[u8]) -> Result<()> {
        let guest_function_call = GuestFunctionCall::try_from(buffer)?;
        let layout = self.layout;
        guest_function_call.write_to_memory(self.get_shared_mem_mut(), &layout)
    }
}
