use std::mem::{offset_of, size_of};

use hyperlight_common::mem::{GuestStackData, HyperlightPEB, PAGE_SIZE_USIZE};
use paste::paste;
use rand::rngs::OsRng;
use rand::RngCore;
use tracing::{instrument, Span};

use super::memory_region::{MemoryRegion, MemoryRegionFlags, MemoryRegionVecBuilder};
#[cfg(test)]
use super::ptr::HostPtr;
use super::shared_mem::SharedMemory;
use crate::error::HyperlightError::{GuestOffsetIsInvalid, MemoryRequestTooBig};
use crate::sandbox::SandboxConfiguration;
use crate::Result;

// +-------------------------------------------+
// |               Guest Stack                 |
// +-------------------------------------------+
// |             Guard Page (4KiB)             |
// +-------------------------------------------+
// |             Guest Heap                    |
// +-------------------------------------------+
// |         Guest Panic Context               |
// +-------------------------------------------+
// |             Output Data                   |
// +-------------------------------------------+
// |              Input Data                   |
// +-------------------------------------------+
// |           Guest Error Log                 |
// +-------------------------------------------+
// |        Host Exception Handlers            |
// +-------------------------------------------+
// |        Host Function Definitions          |
// +-------------------------------------------+
// |                PEB Struct (0x98)          |
// +-------------------------------------------+
// |               Guest Code                  |
// +-------------------------------------------+ 0x203_000
// |                    PD                     |
// +-------------------------------------------+ 0x202_000
// |                   PDPT                    |
// +-------------------------------------------+ 0x201_000
// |                   PML4                    |
// +-------------------------------------------+ 0x200_000
// |                    ⋮                      |
// |                 Unmapped                  |
// |                    ⋮                      |
// +-------------------------------------------+ 0x0

///
/// - `HostDefinitions` - the length of this is the `HostFunctionDefinitionSize`
/// field from `SandboxConfiguration`
///
/// - `HostExceptionData` - memory that contains details of any Host Exception that
/// occurred in outb function. it contains a 32 bit length following by a json
/// serialisation of any error that occurred. the length of this field is
/// `HostExceptionSize` from` `SandboxConfiguration`
///
/// - `GuestError` - contains a buffer for any guest error that occurred.
/// the length of this field is `GuestErrorBufferSize` from `SandboxConfiguration`
///
/// - `InputData` -  this is a buffer that is used for input data to the host program.
/// the length of this field is `InputDataSize` from `SandboxConfiguration`
///
/// - `OutputData` - this is a buffer that is used for output data from host program.
/// the length of this field is `OutputDataSize` from `SandboxConfiguration`
///
/// - `GuestHeap` - this is a buffer that is used for heap data in the guest. the length
/// of this field is returned by the `heap_size()` method of this struct
///
/// - `GuestStack` - this is a buffer that is used for stack data in the guest. the length
/// of this field is returned by the `stack_size()` method of this struct. in reality,
/// the stack might be slightly bigger or smaller than this value since total memory
/// size is rounded up to the nearest 4K, and there is a 16-byte stack guard written
/// to the top of the stack.
///
/// - `GuestPanicContext` - contains a buffer for context associated with any guest
/// panic that occurred.
/// the length of this field is returned by the `guest_panic_context_size()` fn of this struct.

#[derive(Copy, Clone, Debug)]
//TODO:(#1029) Once we have a complete C API, we can restrict visibility to crate level.
pub struct SandboxMemoryLayout {
    pub(super) sandbox_memory_config: SandboxConfiguration,
    /// The stack size of this sandbox.
    pub(super) stack_size: usize,
    /// The heap size of this sandbox.
    pub(super) heap_size: usize,

    /// The following fields are offsets to the actual PEB struct fields.
    /// They are used when writing the PEB struct itself
    peb_offset: usize,
    peb_security_cookie_seed_offset: usize,
    peb_guest_dispatch_function_ptr_offset: usize, // set by guest in guest entrypoint
    pub(super) peb_host_function_definitions_offset: usize,
    pub(crate) peb_host_exception_offset: usize,
    peb_guest_error_offset: usize,
    peb_code_and_outb_pointer_offset: usize,
    peb_input_data_offset: usize,
    peb_output_data_offset: usize,
    peb_guest_panic_context_offset: usize,
    peb_heap_data_offset: usize,
    peb_stack_data_offset: usize,

    // The following are the actual values
    // that are written to the PEB struct
    pub(crate) host_function_definitions_buffer_offset: usize,
    pub(crate) host_exception_buffer_offset: usize,
    pub(super) guest_error_buffer_offset: usize,
    pub(super) input_data_buffer_offset: usize,
    pub(super) output_data_buffer_offset: usize,
    guest_panic_context_buffer_offset: usize,
    guest_heap_buffer_offset: usize,
    guard_page_offset: usize,
    guest_stack_buffer_offset: usize, // the lowest address of the stack

    // other
    pub(crate) peb_address: usize,
    code_size: usize,
}
impl SandboxMemoryLayout {
    /// Four Kilobytes (16^3 bytes) - used to round the total amount of memory
    /// used to the nearest 4K
    const FOUR_K: usize = 0x1000;
    /// The size of the page table within a sandbox
    const PAGE_TABLE_SIZE: usize = 0x3000;
    /// The offset into the sandbox's memory where the PML4 Table is located.
    /// See https://www.pagetable.com/?p=14 for more information.
    pub(crate) const PML4_OFFSET: usize = 0x0000;
    /// The offset into the sandbox's memory where the Page Directory Pointer
    /// Table starts.
    pub(super) const PDPT_OFFSET: usize = 0x1000;
    /// The offset into the sandbox's memory where the Page Directory starts.
    pub(super) const PD_OFFSET: usize = 0x2000;
    /// The address (not the offset) to the start of the page directory
    pub(super) const PD_GUEST_ADDRESS: usize = Self::BASE_ADDRESS + Self::PD_OFFSET;
    /// The address (not the offset) into sandbox memory where the Page
    /// Directory Pointer Table starts
    pub(super) const PDPT_GUEST_ADDRESS: usize = Self::BASE_ADDRESS + Self::PDPT_OFFSET;
    /// The offset into the sandbox's memory where code starts.
    pub(super) const CODE_OFFSET: usize = Self::PAGE_TABLE_SIZE;
    /// The maximum amount of memory a single sandbox will be allowed.
    /// The addressable virtual memory with current paging setup is virtual address 0x0 - 0x40000000 (excl.),
    /// However, the memory up to Self::BASE_ADDRESS is not used.
    const MAX_MEMORY_SIZE: usize = 0x40000000 - Self::BASE_ADDRESS;

    /// The base address of the sandbox's memory.
    //TODO:(#1029) Once we have a complete C API, we can restrict visibility to crate level.
    pub const BASE_ADDRESS: usize = 0x0200000;

    /// The absolute address (assuming sandbox memory starts at BASE_ADDRESS) into
    /// sandbox memory where code starts.
    pub(super) const GUEST_CODE_ADDRESS: usize = Self::BASE_ADDRESS + Self::CODE_OFFSET;

    // the offset into a sandbox's input/output buffer where the stack starts
    const STACK_POINTER_SIZE_BYTES: u64 = 8;

    /// Create a new `SandboxMemoryLayout` with the given
    /// `SandboxConfiguration`, code size and stack/heap size.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn new(
        cfg: SandboxConfiguration,
        code_size: usize,
        stack_size: usize,
        heap_size: usize,
    ) -> Result<Self> {
        // The following offsets are to the fields of the PEB struct itself!
        let peb_offset = Self::PAGE_TABLE_SIZE + round_up_to(code_size, Self::FOUR_K);
        let peb_security_cookie_seed_offset =
            peb_offset + offset_of!(HyperlightPEB, security_cookie_seed);
        let peb_guest_dispatch_function_ptr_offset =
            peb_offset + offset_of!(HyperlightPEB, guest_function_dispatch_ptr);
        let peb_host_function_definitions_offset =
            peb_offset + offset_of!(HyperlightPEB, hostFunctionDefinitions);
        let peb_host_exception_offset = peb_offset + offset_of!(HyperlightPEB, hostException);
        let peb_guest_error_offset = peb_offset + offset_of!(HyperlightPEB, guestErrorData);
        let peb_code_and_outb_pointer_offset = peb_offset + offset_of!(HyperlightPEB, pCode);
        let peb_input_data_offset = peb_offset + offset_of!(HyperlightPEB, inputdata);
        let peb_output_data_offset = peb_offset + offset_of!(HyperlightPEB, outputdata);
        let peb_guest_panic_context_offset =
            peb_offset + offset_of!(HyperlightPEB, guestPanicContextData);
        let peb_heap_data_offset = peb_offset + offset_of!(HyperlightPEB, guestheapData);
        let peb_stack_data_offset = peb_offset + offset_of!(HyperlightPEB, gueststackData);

        // The following offsets are the actual values that relate to memory layout,
        // which are written to PEB struct
        let peb_address = Self::BASE_ADDRESS + peb_offset;
        // make sure host function definitions buffer starts at 4K boundary
        let host_function_definitions_buffer_offset = round_up_to(
            peb_stack_data_offset + size_of::<GuestStackData>(),
            Self::FOUR_K,
        );
        // make sure host exception buffer starts at 4K boundary
        let host_exception_buffer_offset = round_up_to(
            host_function_definitions_buffer_offset + cfg.get_host_function_definition_size(),
            Self::FOUR_K,
        );
        let guest_error_buffer_offset =
            host_exception_buffer_offset + cfg.get_host_exception_size();
        let input_data_buffer_offset =
            guest_error_buffer_offset + cfg.get_guest_error_buffer_size();
        let output_data_buffer_offset = input_data_buffer_offset + cfg.get_input_data_size();
        let guest_panic_context_buffer_offset =
            output_data_buffer_offset + cfg.get_output_data_size();
        // make sure heap buffer starts at 4K boundary
        let guest_heap_buffer_offset = round_up_to(
            guest_panic_context_buffer_offset + cfg.get_guest_panic_context_buffer_size(),
            Self::FOUR_K,
        );
        // make sure guard page starts at 4K boundary
        let guard_page_offset = round_up_to(guest_heap_buffer_offset + heap_size, Self::FOUR_K);
        let guest_stack_buffer_offset = guard_page_offset + Self::FOUR_K;
        // round up stack size to page size. This is needed for MemoryRegion
        let stack_size_rounded = round_up_to(stack_size, Self::FOUR_K);

        Ok(Self {
            peb_offset,
            stack_size: stack_size_rounded,
            heap_size,
            peb_security_cookie_seed_offset,
            peb_guest_dispatch_function_ptr_offset,
            peb_host_function_definitions_offset,
            peb_host_exception_offset,
            peb_guest_error_offset,
            peb_code_and_outb_pointer_offset,
            peb_input_data_offset,
            peb_output_data_offset,
            peb_guest_panic_context_offset,
            peb_heap_data_offset,
            peb_stack_data_offset,
            guest_error_buffer_offset,
            sandbox_memory_config: cfg,
            code_size,
            host_function_definitions_buffer_offset,
            host_exception_buffer_offset,
            input_data_buffer_offset,
            output_data_buffer_offset,
            guest_heap_buffer_offset,
            guest_stack_buffer_offset,
            peb_address,
            guest_panic_context_buffer_offset,
            guard_page_offset,
        })
    }

    /// Get the offset in guest memory to the size field in the
    /// `HostExceptionData` structure.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_host_exception_size_offset(&self) -> usize {
        // The size field is the first field in the `HostExceptionData` struct
        self.peb_host_exception_offset
    }

    /// Get the offset in guest memory to the max size of the guest error buffer
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_guest_error_buffer_size_offset(&self) -> usize {
        self.peb_guest_error_offset
    }

    /// Get the offset in guest memory to the error message buffer pointer
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_guest_error_buffer_pointer_offset(&self) -> usize {
        self.peb_guest_error_offset + size_of::<u64>()
    }

    /// Get the offset in guest memory to the output data size
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_output_data_size_offset(&self) -> usize {
        // The size field is the first field in the `OutputData` struct
        self.peb_output_data_offset
    }

    /// Get the offset in guest memory to the host function definitions
    /// size
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_host_function_definitions_size_offset(&self) -> usize {
        // The size field is the first field in the `HostFunctions` struct
        self.peb_host_function_definitions_offset
    }

    /// Get the offset in guest memory to the host function definitions
    /// pointer.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_host_function_definitions_pointer_offset(&self) -> usize {
        // The size field is the field after the size field in the `HostFunctions` struct which is a u64
        self.peb_host_function_definitions_offset + size_of::<u64>()
    }

    /// Get the offset in guest memory to the minimum guest stack address.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_min_guest_stack_address_offset(&self) -> usize {
        // The minimum guest stack address is the start of the guest stack
        self.peb_stack_data_offset
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_stack_size(&self) -> usize {
        self.stack_size
    }

    /// Get the offset in guest memory to the start of host errors
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_host_exception_offset(&self) -> usize {
        self.host_exception_buffer_offset
    }

    /// Get the address of the code section on the host, given `share_mem`'s
    /// base address and whether or not Hyperlight is executing with in-memory
    /// mode enabled.
    #[cfg(test)]
    pub(crate) fn get_host_code_address(shared_mem: &SharedMemory) -> Result<HostPtr> {
        HostPtr::try_from((Self::CODE_OFFSET, shared_mem))
    }

    /// Get the offset in guest memory to the OutB pointer.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_outb_pointer_offset(&self) -> usize {
        // The outb pointer is immediately after the code pointer
        // in the `CodeAndOutBPointers` struct which is a u64
        self.peb_code_and_outb_pointer_offset + size_of::<u64>()
    }

    #[cfg(target_os = "windows")]
    /// Get the offset in guest memory to the OutB context.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_outb_context_offset(&self) -> usize {
        // The outb context is immediately after the outb pointer
        // in the `CodeAndOutBPointers` struct which is a u64
        self.get_outb_pointer_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the output data pointer.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_output_data_pointer_offset(&self) -> usize {
        // This field is immedaitely after the output data size field,
        // which is a `u64`.
        self.get_output_data_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the start of output data.
    ///
    /// This function exists to accommodate the macro that generates C API
    /// compatible functions.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_output_data_offset(&self) -> usize {
        self.output_data_buffer_offset
    }

    /// Get the offset in guest memory to the input data size.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_input_data_size_offset(&self) -> usize {
        // The input data size is the first field in the `InputData` struct
        self.peb_input_data_offset
    }

    /// Get the offset in guest memory to the input data pointer.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_input_data_pointer_offset(&self) -> usize {
        // The input data pointer is immediately after the input
        // data size field in the `InputData` struct which is a `u64`.
        self.get_input_data_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the code pointer
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_code_pointer_offset(&self) -> usize {
        // The code pointer is the first field
        // in the `CodeAndOutBPointers` struct which is a u64
        self.peb_code_and_outb_pointer_offset
    }

    /// Get the offset in guest memory to where the guest dispatch function
    /// pointer is written
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_dispatch_function_pointer_offset(&self) -> usize {
        self.peb_guest_dispatch_function_ptr_offset
    }

    /// Get the offset in guest memory to the PEB address
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_in_process_peb_offset(&self) -> usize {
        self.peb_offset
    }

    /// Get the offset in guest memory to the heap size
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_heap_size_offset(&self) -> usize {
        self.peb_heap_data_offset
    }

    /// Get the offset of the heap pointer in guest memory,
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_heap_pointer_offset(&self) -> usize {
        // The heap pointer is immediately after the
        // heap size field in the `GuestHeap` struct which is a `u64`.
        self.get_heap_size_offset() + size_of::<u64>()
    }

    /// Get the offset to the top of the stack in guest memory
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_top_of_stack_offset(&self) -> usize {
        self.guest_stack_buffer_offset
    }

    // Get the offset in guest memory to the start of the guest panic context data
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_guest_panic_context_offset(&self) -> usize {
        self.peb_guest_panic_context_offset
    }

    // Get the offset to the guest panic context buffer size
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_guest_panic_context_size_offset(&self) -> usize {
        // The size field is the first field in the `GuestPanicContext` data
        self.peb_guest_panic_context_offset
    }

    /// Get the offset to the guest panic context buffer pointer
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_guest_panic_context_buffer_pointer_offset(&self) -> usize {
        // The guest panic data pointer is immediately after the guest
        // panic data size field in the `GuestPanicCOntext` data which is a `u64`
        self.get_guest_panic_context_size_offset() + size_of::<u64>()
    }

    /// Get the offset to the guest panic context buffer pointer
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_guest_panic_context_buffer_offset(&self) -> usize {
        self.guest_panic_context_buffer_offset
    }

    /// Get the offset to the guest guard page
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn get_guard_page_offset(&self) -> usize {
        self.guard_page_offset
    }

    /// Get the total size of guest memory in `self`'s memory
    /// layout.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_unaligned_memory_size(&self) -> usize {
        self.get_top_of_stack_offset() + self.get_stack_size()
    }

    /// Get the total size of guest memory in `self`'s memory
    /// layout aligned to 4k page boundaries.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_memory_size(&self) -> Result<usize> {
        let total_memory = self.get_unaligned_memory_size();

        // Size should be a multiple of 4K.
        let remainder = total_memory % Self::FOUR_K;
        let multiples = total_memory / Self::FOUR_K;
        let size = match remainder {
            0 => total_memory,
            _ => (multiples + 1) * Self::FOUR_K,
        };

        if size > Self::MAX_MEMORY_SIZE {
            Err(MemoryRequestTooBig(size, Self::MAX_MEMORY_SIZE))
        } else {
            Ok(size)
        }
    }

    /// Returns the memory regions associated with this memory layout,
    /// suitable for passing to a hypervisor for mapping into memory
    pub fn get_memory_regions(&self, shared_mem: &SharedMemory) -> Vec<MemoryRegion> {
        let mut builder = MemoryRegionVecBuilder::new(Self::BASE_ADDRESS, shared_mem.base_addr());
        // PML4, PDPT, PD
        let code_offset = builder.push_page_aligned(
            Self::PAGE_TABLE_SIZE,
            MemoryRegionFlags::READ | MemoryRegionFlags::WRITE,
        );
        assert_eq!(code_offset, Self::CODE_OFFSET);

        // code
        let peb_offset = builder.push_page_aligned(
            self.code_size,
            MemoryRegionFlags::READ | MemoryRegionFlags::WRITE | MemoryRegionFlags::EXECUTE,
        );
        assert_eq!(
            peb_offset,
            TryInto::<usize>::try_into(self.peb_offset).unwrap()
        );

        // PEB
        let host_functions_definitions_offset = builder.push_page_aligned(
            size_of::<HyperlightPEB>(),
            MemoryRegionFlags::READ | MemoryRegionFlags::WRITE,
        );
        assert_eq!(
            host_functions_definitions_offset,
            TryInto::<usize>::try_into(self.host_function_definitions_buffer_offset).unwrap()
        );

        // host function definitions
        let host_exception_offset = builder.push_page_aligned(
            self.sandbox_memory_config
                .get_host_function_definition_size(),
            MemoryRegionFlags::READ,
        );
        assert_eq!(
            host_exception_offset,
            TryInto::<usize>::try_into(self.host_exception_buffer_offset).unwrap()
        );

        // host exception, guest error, guest input data, guest output, guest panic context
        let heap_offset = builder.push_page_aligned(
            self.sandbox_memory_config.get_host_exception_size()
                + self.sandbox_memory_config.get_guest_error_buffer_size()
                + self.sandbox_memory_config.get_input_data_size()
                + self.sandbox_memory_config.get_output_data_size()
                + self
                    .sandbox_memory_config
                    .get_guest_panic_context_buffer_size(),
            MemoryRegionFlags::READ | MemoryRegionFlags::WRITE,
        );
        assert_eq!(
            heap_offset,
            TryInto::<usize>::try_into(self.guest_heap_buffer_offset).unwrap()
        );

        // heap
        #[cfg(feature = "executable_heap")]
        let guard_page_offset = builder.push_page_aligned(
            self.heap_size,
            MemoryRegionFlags::READ | MemoryRegionFlags::WRITE | MemoryRegionFlags::EXECUTE,
        );
        #[cfg(not(feature = "executable_heap"))]
        let guard_page_offset = builder.push_page_aligned(
            self.heap_size,
            MemoryRegionFlags::READ | MemoryRegionFlags::WRITE,
        );

        assert_eq!(
            guard_page_offset,
            TryInto::<usize>::try_into(self.guard_page_offset).unwrap()
        );

        // guard page
        let stack_offset = builder.push_page_aligned(PAGE_SIZE_USIZE, MemoryRegionFlags::READ);
        assert_eq!(
            stack_offset,
            TryInto::<usize>::try_into(self.guest_stack_buffer_offset).unwrap()
        );

        // stack
        let final_offset = builder.push_page_aligned(
            self.get_stack_size(),
            MemoryRegionFlags::READ | MemoryRegionFlags::WRITE,
        );
        assert_eq!(final_offset, self.get_unaligned_memory_size());
        builder.build()
    }

    /// Write the finished memory layout to `shared_mem` and return
    /// `Ok` if successful.
    ///
    /// Note: `shared_mem` may have been modified, even if `Err` was returned
    /// from this function.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn write(
        &self,
        shared_mem: &mut SharedMemory,
        guest_offset: usize,
        size: usize,
    ) -> Result<()> {
        macro_rules! get_address {
            ($something:ident) => {
                paste! {
                    if guest_offset == 0 {
                        let offset = self.[<$something _offset>];
                        let calculated_addr = shared_mem.calculate_address(offset)?;
                        u64::try_from(calculated_addr)?
                    } else {
                        u64::try_from(guest_offset +  self.[<$something _offset>])?
                    }
                }
            };
        }

        if guest_offset != SandboxMemoryLayout::BASE_ADDRESS
            && guest_offset != shared_mem.base_addr()
        {
            return Err(GuestOffsetIsInvalid(guest_offset));
        }

        // Start of setting up the PEB. The following are in the order of the PEB fields

        // Set up the security cookie seed
        let mut security_cookie_seed = [0u8; 8];
        OsRng.fill_bytes(&mut security_cookie_seed);
        shared_mem.copy_from_slice(&security_cookie_seed, self.peb_security_cookie_seed_offset)?;

        // Skip guest_dispatch_function_ptr_offset because it is set by the guest

        // Set up Host Function Definition
        shared_mem.write_u64(
            self.get_host_function_definitions_size_offset(),
            self.sandbox_memory_config
                .get_host_function_definition_size()
                .try_into()?,
        )?;
        let addr = get_address!(host_function_definitions_buffer);
        shared_mem.write_u64(self.get_host_function_definitions_pointer_offset(), addr)?;

        // Set up Host Exception Header
        // The peb only needs to include the size, not the actual buffer
        // since the the guest wouldn't want to read the buffer anyway
        shared_mem.write_u64(
            self.get_host_exception_size_offset(),
            self.sandbox_memory_config
                .get_host_exception_size()
                .try_into()?,
        )?;

        // Set up Guest Error Fields
        let addr = get_address!(guest_error_buffer);
        shared_mem.write_u64(self.get_guest_error_buffer_pointer_offset(), addr)?;
        shared_mem.write_u64(
            self.get_guest_error_buffer_size_offset(),
            u64::try_from(self.sandbox_memory_config.get_guest_error_buffer_size())?,
        )?;

        // Skip code, is set when loading binary
        // skip outb and outb context, is set when running in_proc

        // Set up input buffer pointer
        shared_mem.write_u64(
            self.get_input_data_size_offset(),
            self.sandbox_memory_config
                .get_input_data_size()
                .try_into()?,
        )?;
        let addr = get_address!(input_data_buffer);
        shared_mem.write_u64(self.get_input_data_pointer_offset(), addr)?;

        // Set up output buffer pointer
        shared_mem.write_u64(
            self.get_output_data_size_offset(),
            self.sandbox_memory_config
                .get_output_data_size()
                .try_into()?,
        )?;
        let addr = get_address!(output_data_buffer);
        shared_mem.write_u64(self.get_output_data_pointer_offset(), addr)?;

        // Set up the guest panic context buffer
        let addr = get_address!(guest_panic_context_buffer);
        shared_mem.write_u64(
            self.get_guest_panic_context_size_offset(),
            self.sandbox_memory_config
                .get_guest_panic_context_buffer_size()
                .try_into()?,
        )?;
        shared_mem.write_u64(self.get_guest_panic_context_buffer_pointer_offset(), addr)?;

        // Set up heap buffer pointer
        let addr = get_address!(guest_heap_buffer);
        shared_mem.write_u64(self.get_heap_size_offset(), self.heap_size.try_into()?)?;
        shared_mem.write_u64(self.get_heap_pointer_offset(), addr)?;

        // Set up Min Guest Stack Address
        shared_mem.write_u64(
            self.get_min_guest_stack_address_offset(),
            (guest_offset + (size - self.stack_size)).try_into()?,
        )?;

        // End of setting up the PEB

        // Initialize the stack pointers of input data and output data
        // to point to the ninth (index 8) byte, which is the first free address
        // of the each respective stack. The first 8 bytes are the stack pointer itself.
        shared_mem.write_u64(
            self.input_data_buffer_offset,
            Self::STACK_POINTER_SIZE_BYTES,
        )?;
        shared_mem.write_u64(
            self.output_data_buffer_offset,
            Self::STACK_POINTER_SIZE_BYTES,
        )?;

        Ok(())
    }
}

fn round_up_to(value: usize, multiple: usize) -> usize {
    (value + multiple - 1) & !(multiple - 1)
}

#[cfg(test)]
mod tests {
    use hyperlight_common::mem::PAGE_SIZE_USIZE;

    use super::*;
    use crate::mem::ptr_offset::Offset;
    use crate::mem::shared_mem::SharedMemory;

    #[test]
    fn test_round_up() {
        assert_eq!(0, round_up_to(0, 4));
        assert_eq!(4, round_up_to(1, 4));
        assert_eq!(4, round_up_to(2, 4));
        assert_eq!(4, round_up_to(3, 4));
        assert_eq!(4, round_up_to(4, 4));
        assert_eq!(8, round_up_to(5, 4));
        assert_eq!(8, round_up_to(6, 4));
        assert_eq!(8, round_up_to(7, 4));
        assert_eq!(8, round_up_to(8, 4));
        assert_eq!(
            SandboxMemoryLayout::FOUR_K,
            round_up_to(44, SandboxMemoryLayout::FOUR_K)
        );
        assert_eq!(
            SandboxMemoryLayout::FOUR_K,
            round_up_to(4095, SandboxMemoryLayout::FOUR_K)
        );
        assert_eq!(
            SandboxMemoryLayout::FOUR_K,
            round_up_to(4096, SandboxMemoryLayout::FOUR_K)
        );
        assert_eq!(
            SandboxMemoryLayout::FOUR_K * 2,
            round_up_to(4097, SandboxMemoryLayout::FOUR_K)
        );
        assert_eq!(
            SandboxMemoryLayout::FOUR_K * 2,
            round_up_to(8191, SandboxMemoryLayout::FOUR_K)
        );
    }

    #[test]
    fn get_host_code_address() {
        let sm = SharedMemory::new(PAGE_SIZE_USIZE).unwrap();
        let hca_in_proc = SandboxMemoryLayout::get_host_code_address(&sm).unwrap();
        let hca_in_vm = SandboxMemoryLayout::get_host_code_address(&sm).unwrap();
        let code_offset: Offset = SandboxMemoryLayout::CODE_OFFSET.try_into().unwrap();
        assert_eq!(hca_in_proc.offset(), code_offset);
        assert_eq!(hca_in_vm.offset(), code_offset);
        assert_eq!(hca_in_proc, hca_in_vm);
    }

    // helper func for testing
    fn get_expected_memory_size(layout: &SandboxMemoryLayout) -> usize {
        let cfg = layout.sandbox_memory_config;
        let mut expected_size = 0;
        // in order of layout
        expected_size += 3 * PAGE_SIZE_USIZE; // 3 page structs
        expected_size += layout.code_size;

        expected_size += round_up_to(size_of::<HyperlightPEB>(), PAGE_SIZE_USIZE);

        expected_size += round_up_to(cfg.get_host_function_definition_size(), PAGE_SIZE_USIZE);

        expected_size += round_up_to(
            cfg.get_host_exception_size()
                + cfg.get_guest_error_buffer_size()
                + cfg.get_input_data_size()
                + cfg.get_output_data_size()
                + cfg.get_guest_panic_context_buffer_size()
                + layout.heap_size,
            PAGE_SIZE_USIZE,
        );

        expected_size += PAGE_SIZE_USIZE; // guard page

        expected_size += round_up_to(layout.stack_size, PAGE_SIZE_USIZE);
        expected_size
    }

    #[test]
    fn test_get_memory_size() {
        let sbox_cfg = SandboxConfiguration::default();
        let sbox_mem_layout = SandboxMemoryLayout::new(sbox_cfg, 4096, 2048, 4096).unwrap();
        assert_eq!(
            sbox_mem_layout.get_memory_size().unwrap(),
            get_expected_memory_size(&sbox_mem_layout)
        );
    }
}
