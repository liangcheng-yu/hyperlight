#[cfg(test)]
use super::ptr::HostPtr;
use super::shared_mem::SharedMemory;
use crate::error::HyperlightError::{GuestOffsetIsInvalid, MemoryRequestTooBig};
use crate::mem::ptr_offset::Offset;
use crate::sandbox::SandboxConfiguration;
use crate::Result;
use paste::paste;
use rand::rngs::OsRng;
use rand::RngCore;
use std::mem::size_of;
use tracing::{instrument, Span};

// The following structs are not used other than to calculate the size of the memory needed
// and also to illustrate the layout of the memory

// the start of the guest memory contains the page tables and is always located at the Virtual Address 0x200000 when running in a Hypervisor:

// Virtual Address
//
// 0x200000    PML4
// 0x201000    PDPT
// 0x202000    PD
// 0x203000    The guest PE code (When the code has been loaded using LoadLibrary to debug the guest this will not be present and code length will be zero;
//
// The pointer passed to the Entrypoint in the Guest application is  0x200000 + size of page table + size of code.
// At the Entrypoint address the structs below are laid out in order

#[repr(C)]
struct GuestSecurityCookie {
    seed: u64,
}

#[repr(C)]
struct GuestDispatchFunctionPointer {
    ptr: u64,
}

#[repr(C)]
struct HostFunctions {
    host_function_definitions_size: u64,
    host_function_definitions: u64,
}

#[repr(C)]
struct HostExceptionData {
    host_exception_size: u64,
}

#[repr(C)]
struct GuestError {
    // This is a pointer to a buffer that contains the details of any guest error that occurred.
    guest_error_buffer: u64,
    // This is the size of the buffer that contains the details of any guest error that occurred.
    guest_error_buffer_size: u64,
}

#[repr(C)]
struct CodeAndOutBPointers {
    // This is a pointer to the code that is to be executed in the guest.
    code_pointer: u64,
    // This is a pointer to the outb function that is used when running in process
    outb_pointer: u64,
    // This is a pointer to the rust object to allow C to callback to the outb function exposed from Rust.
    outb_context: u64,
}

#[repr(C)]
struct InputData {
    input_data_size: u64,
    input_data_buffer: u64,
}

#[repr(C)]
struct OutputData {
    output_data_size: u64,
    output_data_buffer: u64,
}

#[repr(C)]
struct GuestHeap {
    guest_heap_size: u64,
    guest_heap_buffer: u64,
}

#[repr(C)]
struct GuestStack {
    min_guest_stack_address: u64,
}

// Following these structures are the memory buffers as follows:
//
// Host Function definitions - length SandboxConfiguration.HostFunctionDefinitionSize
// Host Exception Details - length SandboxConfiguration.HostExceptionSize , this contains details of any Host Exception that occurred in outb function
// it contains a 32 bit length following by a json serialisation of any error that occurred.
// Guest Error Buffer - length SandboxConfiguration.GuestErrorBufferSize this contains the details of any guest error that occurre, it is serialised and deserialised using flatbuffers.
// Input Data Buffer - length SandboxConfiguration.InputDataSize this is a buffer that is used for input data to the host program
// Output Data Buffer - length SandboxConfiguration.OutputDataSize this is a buffer that is used for output data from host program
// Guest Heap - length heapSize this is a memory buffer provided to the guest to be used as heap memory.
// Guest Stack - length stackSize this is the memory used for the guest stack (in reality the stack may be slightly bigger or smaller as the total memory size is rounded up to nearest 4K and there is a 16 bte stack guard written to the top of the stack).

/// Mostly a collection of utilities organized around the layout of the
/// memory in a sandbox.
///
/// The memory is laid out roughly as follows (using other structs in this module
/// for illustration)
///
/// - `HostDefinitions` - the length of this is the `HostFunctionDefinitionSize`
/// field from `SandboxConfiguration`
///
/// - `HostExceptionData` - memory that contains details of any Host Exception that
/// occurred in outb function. it contains a 32 bit length following by a json
/// serialisation of any error that occurred. the length of this field is
/// `HostExceptionSize` from` `SandboxConfiguration`
///
/// - `GuestError` - contains an buffer for any guest error that occurred.
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
///

#[derive(Copy, Clone, Debug)]
//TODO:(#1029) Once we have a complete C API, we can restrict visibility to crate level.
pub struct SandboxMemoryLayout {
    sandbox_memory_config: SandboxConfiguration,
    /// The peb offset into this sandbox.
    peb_offset: Offset,
    /// The stack size of this sandbox.
    pub(super) stack_size: usize,
    /// The heap size of this sandbox.
    pub(super) heap_size: usize,
    /// The offset to the start of host functions within this sandbox.
    host_functions_offset: Offset,
    /// The offset to the start of host exceptions within this sandbox.
    host_exception_offset: Offset,
    /// The offset to the pointer to the guest error buffer within this sandbox.
    guest_error_buffer_pointer_offset: Offset,
    /// The offset to the size of the guest error buffer within this sandbox.
    guest_error_buffer_size_offset: Offset,
    /// The offset to the start of both code and the outb function
    /// pointers within this sandbox.
    code_and_outb_pointer_offset: Offset,
    /// The offset to the start of input data within this sandbox.
    input_data_offset: Offset,
    /// The offset to the start of output data within this sandbox.
    output_data_offset: Offset,
    /// The offset to the start of the guest heap within this sandbox.
    heap_data_offset: Offset,
    /// The offset to the start of the guest stack within this sandbox.
    stack_data_offset: Offset,
    /// The size of code inside this sandbox.
    code_size: usize,
    /// The offset to the start of the definitions of host functions inside
    /// this sandbox.
    pub(super) host_function_definitions_offset: Offset,
    /// The offset to the start of the buffer for host exceptions inside
    /// this sandbox.
    host_exception_buffer_offset: Offset,
    /// The offset to the start of guest errors inside this sandbox.
    pub(super) guest_error_buffer_offset: Offset,
    /// The offset to the start of the input data buffer inside this
    /// sandbox.
    pub(super) input_data_buffer_offset: Offset,
    /// The offset to the start of the output data buffer inside this
    /// sandbox.
    pub(super) output_data_buffer_offset: Offset,
    /// The offset to the start of the guest heap buffer inside this
    /// sandbox.
    guest_heap_buffer_offset: Offset,
    /// The offset to the start of the guest stack buffer inside this
    /// sandbox.
    guest_stack_buffer_offset: Offset,
    /// The peb address inside this sandbox.
    pub(crate) peb_address: usize,
    /// The offset to the guest security cookie
    guest_security_cookie_seed_offset: Offset,
    /// The offset to the guest dispatch function pointer
    guest_dispatch_function_ptr_offset: Offset,
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
    /// The offset into the sandbox's memory where the Page Directory starts.
    pub(super) const PD_OFFSET: usize = 0x2000;
    /// The address (not the offset) to the start of the page directory
    pub(super) const PD_GUEST_ADDRESS: usize = Self::BASE_ADDRESS + Self::PD_OFFSET;
    /// The offset into the sandbox's memory where the Page Directory Pointer
    /// Table starts.
    pub(super) const PDPT_OFFSET: usize = 0x1000;
    /// The address (not the offset) into sandbox memory where the Page
    /// Directory Pointer Table starts
    pub(super) const PDPT_GUEST_ADDRESS: usize = Self::BASE_ADDRESS + Self::PDPT_OFFSET;
    /// The offset into the sandbox's memory where code starts.
    pub(super) const CODE_OFFSET: usize = Self::PAGE_TABLE_SIZE;
    /// The maximum amount of memory a single sandbox will be allowed.
    const MAX_MEMORY_SIZE: usize = 0x3FEF0000;

    /// The base address of the sandbox's memory.
    //TODO:(#1029) Once we have a complete C API, we can restrict visibility to crate level.
    pub const BASE_ADDRESS: usize = 0x0200000;

    /// The absolute address (assuming sandbox memory starts at BASE_ADDRESS) into
    /// sandbox memory where code starts.
    pub(super) const GUEST_CODE_ADDRESS: usize = Self::BASE_ADDRESS + Self::CODE_OFFSET;

    /// Create a new `SandboxMemoryLayout` with the given
    /// `SandboxConfiguration`, code size and stack/heap size.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn new(
        cfg: SandboxConfiguration,
        code_size: usize,
        stack_size: usize,
        heap_size: usize,
    ) -> Result<Self> {
        let peb_offset = Offset::try_from(Self::PAGE_TABLE_SIZE + code_size)?;
        let guest_security_cookie_seed_offset =
            Offset::try_from(Self::PAGE_TABLE_SIZE + code_size)?;
        let guest_dispatch_function_ptr_offset =
            guest_security_cookie_seed_offset + size_of::<GuestSecurityCookie>();
        let host_functions_offset =
            guest_dispatch_function_ptr_offset + size_of::<GuestDispatchFunctionPointer>();
        let host_exception_offset = host_functions_offset + size_of::<HostFunctions>();
        let guest_error_buffer_pointer_offset =
            host_exception_offset + size_of::<HostExceptionData>();
        let guest_error_buffer_size_offset = guest_error_buffer_pointer_offset + size_of::<u64>();
        let code_and_outb_pointer_offset =
            guest_error_buffer_pointer_offset + size_of::<GuestError>();
        let input_data_offset = code_and_outb_pointer_offset + size_of::<CodeAndOutBPointers>();
        let output_data_offset = input_data_offset + size_of::<InputData>();
        let heap_data_offset = output_data_offset + size_of::<OutputData>();
        let stack_data_offset = heap_data_offset + size_of::<GuestHeap>();
        let peb_address = usize::try_from(Self::BASE_ADDRESS + peb_offset)?;
        let host_function_definitions_offset = stack_data_offset + size_of::<GuestStack>();
        let host_exception_buffer_offset =
            host_function_definitions_offset + cfg.get_host_function_definition_size();
        let guest_error_buffer_offset =
            host_exception_buffer_offset + cfg.get_host_exception_size();
        let input_data_buffer_offset =
            guest_error_buffer_offset + cfg.get_guest_error_buffer_size();
        let output_data_buffer_offset = input_data_buffer_offset + cfg.get_input_data_size();
        let guest_heap_buffer_offset = output_data_buffer_offset + cfg.get_output_data_size();
        let guest_stack_buffer_offset = guest_heap_buffer_offset + heap_size;
        Ok(Self {
            peb_offset,
            stack_size,
            heap_size,
            host_functions_offset,
            host_exception_offset,
            guest_error_buffer_pointer_offset,
            guest_error_buffer_size_offset,
            code_and_outb_pointer_offset,
            input_data_offset,
            output_data_offset,
            heap_data_offset,
            stack_data_offset,
            sandbox_memory_config: cfg,
            code_size,
            host_function_definitions_offset,
            host_exception_buffer_offset,
            guest_error_buffer_offset,
            input_data_buffer_offset,
            output_data_buffer_offset,
            guest_heap_buffer_offset,
            guest_stack_buffer_offset,
            peb_address,
            guest_security_cookie_seed_offset,
            guest_dispatch_function_ptr_offset,
        })
    }

    /// Get the offset in guest memory to the size field in the
    /// `HostExceptionData` structure.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_host_exception_size_offset(&self) -> Offset {
        // The size field is the first field in the `HostExceptionData` struct
        self.host_exception_offset
    }

    /// Get the offset in guest memory to the max size of the guest error buffer
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_guest_error_buffer_size_offset(&self) -> Offset {
        self.guest_error_buffer_size_offset
    }

    /// Get the offset in guest memory to the error message buffer pointer
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_guest_error_buffer_pointer_offset(&self) -> Offset {
        self.guest_error_buffer_pointer_offset
    }
    /// Get the offset in guest memory to the output data size
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_output_data_size_offset(&self) -> Offset {
        // The size field is the first field in the `OutputData` struct
        self.output_data_offset
    }

    /// Get the offset in guest memory to the host function definitions
    /// size
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_host_function_definitions_size_offset(&self) -> Offset {
        // The size field is the first field in the `HostFunctions` struct
        self.host_functions_offset
    }

    /// Get the offset in guest memory to the host function definitions
    /// pointer.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_host_function_definitions_pointer_offset(&self) -> Offset {
        // The size field is the field after the size field in the `HostFunctions` struct which is a u64
        self.get_host_function_definitions_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the minimum guest stack address.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_min_guest_stack_address_offset(&self) -> Offset {
        // The minimum guest stack address is the start of the guest stack
        self.stack_data_offset
    }

    /// Get the offset in guest memory to the start of host errors
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_host_exception_offset(&self) -> Offset {
        self.host_exception_buffer_offset
    }

    /// Get the address of the code section on the host, given `share_mem`'s
    /// base address and whether or not Hyperlight is executing with in-memory
    /// mode enabled.
    #[cfg(test)]
    pub(crate) fn get_host_code_address(shared_mem: &SharedMemory) -> Result<HostPtr> {
        let code_offset: Offset = Self::CODE_OFFSET.try_into()?;
        HostPtr::try_from((code_offset, shared_mem))
    }

    /// Get the offset in guest memory to the OutB pointer.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_outb_pointer_offset(&self) -> Offset {
        // The outb pointer is immediately after the code pointer
        // in the `CodeAndOutBPointers` struct which is a u64
        self.code_and_outb_pointer_offset + size_of::<u64>()
    }

    #[cfg(target_os = "windows")]
    /// Get the offset in guest memory to the OutB context.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_outb_context_offset(&self) -> Offset {
        // The outb context is immediately after the outb pointer
        // in the `CodeAndOutBPointers` struct which is a u64
        self.get_outb_pointer_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the output data pointer.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_output_data_pointer_offset(&self) -> Offset {
        // This field is immedaitely after the output data size field,
        // which is a `u64`.
        self.get_output_data_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the start of output data.
    ///
    /// This function exists to accommodate the macro that generates C API
    /// compatible functions.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_output_data_offset(&self) -> Offset {
        self.output_data_buffer_offset
    }

    /// Get the offset in guest memory to the input data size.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_input_data_size_offset(&self) -> Offset {
        // The input data size is the first field in the `InputData` struct
        self.input_data_offset
    }

    /// Get the offset in guest memory to the input data pointer.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_input_data_pointer_offset(&self) -> Offset {
        // The input data pointer is immediately after the input
        // data size field in the `InputData` struct which is a `u64`.
        self.get_input_data_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the code pointer
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_code_pointer_offset(&self) -> Offset {
        // The code pointer is the first field
        // in the `CodeAndOutBPointers` struct which is a u64
        self.code_and_outb_pointer_offset
    }

    /// Get the offset in guest memory to where the guest dispatch function
    /// pointer is written
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_dispatch_function_pointer_offset(&self) -> Offset {
        self.guest_dispatch_function_ptr_offset
    }

    /// Get the offset in guest memory to the PEB address
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_in_process_peb_offset(&self) -> Offset {
        self.peb_offset
    }

    /// Get the offset in guest memory to the heap size
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_heap_size_offset(&self) -> Offset {
        self.heap_data_offset
    }

    /// Get the offset of the heap pointer in guest memory,
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_heap_pointer_offset(&self) -> Offset {
        // The heap pointer is immediately after the
        // heap size field in the `GuestHeap` struct which is a `u64`.
        self.get_heap_size_offset() + size_of::<u64>()
    }

    /// Get the offset to the top of the stack in guest memory
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_top_of_stack_offset(&self) -> Offset {
        self.guest_stack_buffer_offset
    }

    /// Get the total size of guest memory in `self`'s memory
    /// layout.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_memory_size(&self) -> Result<usize> {
        let total_memory = self.code_size
            + Self::PAGE_TABLE_SIZE
            + self
                .sandbox_memory_config
                .get_host_function_definition_size()
            + self.sandbox_memory_config.get_input_data_size()
            + self.sandbox_memory_config.get_output_data_size()
            + self.sandbox_memory_config.get_host_exception_size()
            + self.sandbox_memory_config.get_guest_error_buffer_size()
            + size_of::<GuestSecurityCookie>()
            + size_of::<GuestDispatchFunctionPointer>()
            + size_of::<HostFunctions>()
            + size_of::<HostExceptionData>()
            + size_of::<GuestError>()
            + size_of::<CodeAndOutBPointers>()
            + size_of::<InputData>()
            + size_of::<OutputData>()
            + size_of::<GuestHeap>()
            + size_of::<GuestStack>()
            + self.heap_size
            + self.stack_size;

        // Size should be a multiple of 4K.
        let remainder = total_memory % Self::FOUR_K;
        let multiples = total_memory / Self::FOUR_K;
        let size = match remainder {
            0 => total_memory,
            _ => (multiples + 1) * Self::FOUR_K,
        };

        // For our page table, we only mapped virtual memory up to 0x3FFFFFFF and map each 2 meg
        // virtual chunk to physical addresses 2 megabytes below the virtual address.  Since we
        // map virtual up to 0x3FFFFFFF, the max physical address we handle is 0x3FDFFFFF (or
        // 0x3FEF0000 physical total memory)

        if size > Self::MAX_MEMORY_SIZE {
            Err(MemoryRequestTooBig(size, Self::MAX_MEMORY_SIZE))
        } else {
            Ok(size)
        }
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
                        let offset = Offset::try_from(self.[<$something _offset>])?;
                        let calculated_addr = shared_mem.calculate_address(offset)?;
                        u64::try_from(calculated_addr)?
                    } else {
                        u64::from(guest_offset +  self.[<$something _offset>])
                    }
                }
            };
        }

        if guest_offset != SandboxMemoryLayout::BASE_ADDRESS
            && guest_offset != shared_mem.base_addr()
        {
            return Err(GuestOffsetIsInvalid(guest_offset));
        }

        // Set up Guest Error Fields
        shared_mem.write_u64(
            self.get_guest_error_buffer_size_offset(),
            u64::try_from(self.sandbox_memory_config.get_guest_error_buffer_size())?,
        )?;

        let addr = get_address!(guest_error_buffer);

        shared_mem.write_u64(self.get_guest_error_buffer_pointer_offset(), addr)?;

        // Set up Host Exception Header
        shared_mem.write_u64(
            self.get_host_exception_size_offset(),
            self.sandbox_memory_config
                .get_host_exception_size()
                .try_into()?,
        )?;

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

        let addr = get_address!(guest_heap_buffer);

        // Set up heap buffer pointer
        shared_mem.write_u64(self.get_heap_size_offset(), self.heap_size.try_into()?)?;
        shared_mem.write_u64(self.get_heap_pointer_offset(), addr)?;

        let addr = get_address!(host_function_definitions);

        // Set up Host Function Definition
        shared_mem.write_u64(
            self.get_host_function_definitions_size_offset(),
            self.sandbox_memory_config
                .get_host_function_definition_size()
                .try_into()?,
        )?;
        shared_mem.write_u64(self.get_host_function_definitions_pointer_offset(), addr)?;

        // Set up Min Guest Stack Address
        shared_mem.write_u64(
            self.get_min_guest_stack_address_offset(),
            (guest_offset + (size - self.stack_size)).try_into()?,
        )?;

        // Set up the security cookie seed

        let mut security_cookie_seed = [0u8; 8];
        OsRng.fill_bytes(&mut security_cookie_seed);

        shared_mem.copy_from_slice(
            &security_cookie_seed,
            self.guest_security_cookie_seed_offset,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::mem::{ptr_offset::Offset, shared_mem::SharedMemory};

    use super::SandboxMemoryLayout;

    #[test]
    fn get_host_code_address() {
        let sm = SharedMemory::new(100).unwrap();
        let hca_in_proc = SandboxMemoryLayout::get_host_code_address(&sm).unwrap();
        let hca_in_vm = SandboxMemoryLayout::get_host_code_address(&sm).unwrap();
        let code_offset: Offset = SandboxMemoryLayout::CODE_OFFSET.try_into().unwrap();
        assert_eq!(hca_in_proc.offset(), code_offset);
        assert_eq!(hca_in_vm.offset(), code_offset);
        assert_eq!(hca_in_proc, hca_in_vm);
    }
}
