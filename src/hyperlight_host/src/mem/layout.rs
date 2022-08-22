use super::{config::SandboxMemoryConfiguration, guest_mem::GuestMemory};
use anyhow::{anyhow, Result};
use std::mem::size_of;

#[repr(C)]
struct HostFunctionDefinitions {
    func_definition_size: u32,
    func_definitions: usize,
}

#[repr(C)]
struct HostExceptionData {
    host_exception_size: u32,
}

#[repr(C)]
struct GuestError {
    guest_error_code: i64,
    max_message_size: u32,
    message: usize,
}

#[repr(C)]
struct CodeAndOutBPointers {
    code_pointer: usize,
    outb_pointer: usize,
}

#[repr(C)]
struct InputData {
    input_data_size: u32,
    input_data_buffer: usize,
}

#[repr(C)]
struct OutputData {
    output_data_size: u32,
    output_data_buffer: usize,
}

#[repr(C)]
struct GuestHeap {
    guest_heap_size: u32,
    guest_heap_buffer: usize,
}

#[repr(C)]
struct GuestStack {
    min_guest_stack_address: usize,
}

/// Mostly a collection of utilities organized around the layout of the
/// memory in a sandbox.
///
/// The memory is laid out roughly as follows (using other structs in this module
/// for illustration)
///
/// - `HostDefinitions` - the length of this is the `HostFunctionDefinitionSize`
/// field from `SandboxMemoryConfiguration`
///
/// - `HostExceptionData` - memory that contains details of any Host Exception that
/// occurred in outb function. it contains a 32 bit length following by a json
/// serialisation of any error that occurred. the length of this field is
/// `HostExceptionSize` from` `SandboxMemoryConfiguration`
///
/// - `GuestError` - contains an error message string for any guest error that occurred.
/// the length of this field is `GuestErrorMessageSize` from `SandboxMemoryConfiguration`
///
/// - `InputData` -  this is a buffer that is used for input data to the host program.
/// the length of this field is `InputDataSize` from `SandboxMemoryConfiguration`
///
/// - `OutputData` - this is a buffer that is used for output data from host program.
/// the length of this field is `OutputDataSize` from `SandboxMemoryConfiguration`
///
/// - `GuestHeap` - this is a buffer that is used for heap data in the guest. the length
/// of this field is returned by the `heap_size()` method of this struct
///
/// - `GuestStack` - this is a buffer that is used for stack data in the guest. the length
/// of this field is returned by the `stack_size()` method of this struct. in reality,
/// the stack might be slightly bigger or smaller than this value since total memory
/// size is rounded up to the nearest 4K, and there is a 16-byte stack guard written
/// to the top of the stack.
#[derive(Copy, Clone, Debug)]
pub struct SandboxMemoryLayout {
    /// The peb offset into this sandbox.
    pub peb_offset: usize,
    /// The stack size of this sandbox.
    pub stack_size: usize,
    /// The heap size of this sandbox.
    pub heap_size: usize,
    /// The offset to the start of host functions within this sandbox.
    pub host_functions_offset: usize,
    /// The offset to the start of host exceptions within this sandbox.
    pub host_exception_offset: usize,
    /// The offset to the start of guest errors within this sandbox.
    pub guest_error_message_offset: usize,
    /// The offset to the start of both code and the outb function
    /// pointers within this sandbox.
    pub code_and_outb_pointer_offset: usize,
    /// The offset to the start of input data within this sandbox.
    pub input_data_offset: usize,
    /// The offset to the start of output data within this sandbox.
    pub output_data_offset: usize,
    /// The offset to the start of the guest heap within this sandbox.
    pub heap_data_offset: usize,
    /// The offset to the start of the guest stack within this sandbox.
    pub stack_data_offset: usize,
    sandbox_memory_config: SandboxMemoryConfiguration,
    /// The size of code inside this sandbox.
    pub code_size: usize,
    /// The offset to the start of the buffer for host functions inside
    /// this sandbox.
    pub host_functions_buffer_offset: usize,
    /// The offset to the start of the buffer for host exceptions inside
    /// this sandbox.
    pub host_exception_buffer_offset: usize,
    /// The offset to the start of guest errors inside this sandbox.
    pub guest_error_message_buffer_offset: usize,
    /// The offset to the start of the input data buffer inside this
    /// sandbox.
    pub input_data_buffer_offset: usize,
    /// The offset to the start of the output data buffer inside this
    /// sandbox.
    pub output_data_buffer_offset: usize,
    /// The offset to the start of the guest heap buffer inside this
    /// sandbox.
    pub guest_heap_buffer_offset: usize,
    /// The offset to the start of the guest stack buffer inside this
    /// sandbox.
    pub guest_stack_buffer_offset: usize,
    /// The peb address inside this sandbox.
    pub peb_address: usize,
    /// The offset to the guest security cookie
    pub guest_security_cookie_seed_offset: usize,
}

impl SandboxMemoryLayout {
    /// Four Kilobytes (16^3 bytes) - used to round the total amount of memory
    /// used to the nearest 4K
    const FOUR_K: usize = 0x1000;

    /// The size of the page table within a sandbox
    const PAGE_TABLE_SIZE: usize = 0x3000;

    /// The offset into the sandbox's memory where the Page Directory starts.
    /// See https://www.pagetable.com/?p=14 for more information.
    pub const PD_OFFSET: usize = 0x2000;

    /// The offset into the sandbox's memory where the Page Directory Pointer
    /// Table is located.
    pub const PDPT_OFFSET: usize = 0x1000;

    /// The offset into the sandbox's memory where code starts.
    pub const CODE_OFFSET: usize = Self::PAGE_TABLE_SIZE;

    /// The maximum amount of memory a single sandbox will be allowed.
    const MAX_MEMORY_SIZE: usize = 0x3FEF0000;

    /// The base address at which sandboxed code assumes it will load
    pub const BASE_ADDRESS: usize = 0x00200000;

    //// The absolute address (assuming code is loaded at `BASE_ADDRESS`) into
    /// sandbox memory where the page map level 4 starts.
    pub const PML4_GUEST_ADDRESS: usize = Self::BASE_ADDRESS;

    /// The absolute address (assuming sandbox memory starts at BASE_ADDRESS) into
    /// sandbox memory where the Page Directory Pointer Table starts.
    pub const PDPT_GUEST_ADDRESS: usize = Self::BASE_ADDRESS + Self::PDPT_OFFSET;

    /// The absolute address (assuming sandbox memory starts at BASE_ADDRESS) into
    /// sandbox meory where the page directory is located.
    pub const PD_GUEST_ADDRESS: usize = Self::BASE_ADDRESS + Self::PD_OFFSET;

    /// The absolute address (assuming sandbox memory starts at BASE_ADDRESS) into
    /// sandbox memory where code starts.
    pub const GUEST_CODE_ADDRESS: usize = Self::BASE_ADDRESS + Self::CODE_OFFSET;

    /// Create a new `SandboxMemoryLayout` with the given
    /// `SandboxMemoryConfiguration`, code size and stack/heap size.
    pub fn new(
        cfg: SandboxMemoryConfiguration,
        code_size: usize,
        stack_size: usize,
        heap_size: usize,
    ) -> Self {
        // let
        // this.sandboxMemoryConfiguration = sandboxMemoryConfiguration;
        // this.codeSize = codeSize;
        // this.stackSize = stackSize;
        // this.heapSize = heapSize;
        let peb_offset = Self::PAGE_TABLE_SIZE + code_size;
        let host_functions_offset = Self::PAGE_TABLE_SIZE + code_size;
        let host_exception_offset = host_functions_offset + size_of::<HostFunctionDefinitions>();
        let guest_error_message_offset = host_exception_offset + size_of::<HostExceptionData>();
        let code_and_outb_pointer_offset = guest_error_message_offset + size_of::<GuestError>();
        let input_data_offset = code_and_outb_pointer_offset + size_of::<CodeAndOutBPointers>();
        let output_data_offset = input_data_offset + size_of::<InputData>();
        let heap_data_offset = output_data_offset + size_of::<OutputData>();
        let stack_data_offset = heap_data_offset + size_of::<GuestHeap>();
        let peb_address = Self::BASE_ADDRESS + peb_offset;
        let host_functions_buffer_offset = stack_data_offset + size_of::<GuestStack>();
        let host_exception_buffer_offset =
            host_functions_buffer_offset + cfg.host_function_definition_size;
        let guest_error_message_buffer_offset =
            host_exception_buffer_offset + cfg.host_exception_size;
        let input_data_buffer_offset =
            guest_error_message_buffer_offset + cfg.guest_error_message_size;
        let output_data_buffer_offset = input_data_buffer_offset + cfg.input_data_size;
        let guest_heap_buffer_offset = output_data_buffer_offset + cfg.output_data_size;
        let guest_stack_buffer_offset = guest_heap_buffer_offset + heap_size;
        let guest_security_cookie_seed_offset = Self::PAGE_TABLE_SIZE + code_size;
        Self {
            peb_offset,
            stack_size,
            heap_size,
            host_functions_offset,
            host_exception_offset,
            guest_error_message_offset,
            code_and_outb_pointer_offset,
            input_data_offset,
            output_data_offset,
            heap_data_offset,
            stack_data_offset,
            sandbox_memory_config: cfg,
            code_size,
            host_functions_buffer_offset,
            host_exception_buffer_offset,
            guest_error_message_buffer_offset,
            input_data_buffer_offset,
            output_data_buffer_offset,
            guest_heap_buffer_offset,
            guest_stack_buffer_offset,
            peb_address,
            guest_security_cookie_seed_offset,
        }
    }
    fn get_guest_error_message_offset(&self) -> usize {
        self.guest_error_message_buffer_offset
    }

    /// Get the offset in guest memory to the start of guest errors.
    pub fn get_guest_error_offset(&self) -> usize {
        self.guest_error_message_offset
    }

    /// Get the offset in guest memory to the size field in the
    /// guest error message buffer.
    ///
    /// This is the field after the `GuestErrorMessage` field,
    /// which is a `usize`
    pub fn get_guest_error_message_size_offset(&self) -> usize {
        self.get_guest_error_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the error message pointer.
    ///
    /// This offset is after the message size, which is a `usize`.
    pub fn get_guest_error_message_pointer_offset(&self) -> usize {
        self.get_guest_error_message_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the start of function
    /// definitions.
    pub fn get_function_definition_offset(&self) -> usize {
        self.host_functions_buffer_offset
    }

    /// Get the offset in guest memory to the output data size
    pub fn get_output_data_size_offset(&self) -> usize {
        self.output_data_offset
    }

    /// Get the offset in guest memory to the function definition
    /// size
    fn get_function_definition_size_offset(&self) -> usize {
        self.host_functions_offset
    }

    /// Get the offset in guest memory to the function definition
    /// pointer.
    pub fn get_function_definition_pointer_offset(&self) -> usize {
        self.get_function_definition_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the minimum guest stack
    /// address pointer.
    pub fn get_min_guest_stack_address_offset(&self) -> usize {
        self.stack_data_offset
    }

    fn get_host_exception_size_offset(&self) -> usize {
        self.host_exception_offset
    }

    /// Get the offset in guest memory to the start of host exceptions
    pub fn get_host_exception_offset(&self) -> usize {
        self.host_exception_buffer_offset
    }

    /// Get the offset in guest memory to the OutB pointer.
    ///
    /// The outb pointer is immediately after the code pointer,
    /// which is a u64
    pub fn get_out_b_pointer_offset(&self) -> usize {
        self.code_and_outb_pointer_offset + size_of::<u64>()
    }

    /// Get the offset in guest memory to the output data pointer.
    ///
    /// This field is immedaitely after the output data size field,
    /// which is a `u64`.
    pub fn get_output_data_pointer_offset(&self) -> usize {
        self.get_output_data_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the start of output data.
    pub fn get_output_data_offset(&self) -> usize {
        self.output_data_buffer_offset
    }

    /// Get the offset in guest memory to the input data size.
    pub fn get_input_data_size_offset(&self) -> usize {
        self.input_data_offset
    }

    /// Get the offset in guest memory to the input data pointer.
    ///
    /// This input data pointer is immediately after the input
    /// data size field, which is a `u64`.
    pub fn get_input_data_pointer_offset(&self) -> usize {
        self.get_input_data_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the start of the input data
    /// buffer.
    pub fn get_input_data_offset(&self) -> usize {
        self.input_data_offset
    }

    /// Get the offset in guest memory to the code pointer
    pub fn get_code_pointer_offset(&self) -> usize {
        self.code_and_outb_pointer_offset
    }

    /// Get the offset in guest memory to the dispatch function
    /// pointer.
    pub fn get_dispatch_function_pointer_offset(&self) -> usize {
        self.get_function_definition_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the PEB address
    pub fn get_in_process_peb_offset(&self) -> usize {
        self.peb_offset
    }

    /// Get the offset in guest memory to the heap size
    pub fn get_heap_size_offset(&self) -> usize {
        self.heap_data_offset
    }

    /// Get the offset in  of the heap pointer in guest memory,
    /// given `addr` as the base address.
    pub fn get_heap_pointer_offset(&self) -> usize {
        self.get_heap_size_offset() + size_of::<u64>()
    }

    /// Get the offset to the heap in guest memory
    pub fn get_heap_offset(&self) -> usize {
        self.guest_heap_buffer_offset
    }

    /// Get the offset to the top of the stack in guest memory
    pub fn get_top_of_stack_offset(&self) -> usize {
        self.guest_stack_buffer_offset
    }

    /// Get the offset in guest memory to the pml4 table.
    pub fn get_host_pml4_offset() -> usize {
        0
    }

    /// Get the offset in guest memory to the PDPT address
    pub fn get_host_pdpt_offset() -> usize {
        Self::PDPT_OFFSET
    }

    /// Get the offset to the page descriptor in guest memory
    pub fn get_host_pd_offset() -> usize {
        Self::PD_OFFSET
    }

    /// Get the offset to code in guest memory
    pub fn get_host_code_offset() -> usize {
        Self::CODE_OFFSET
    }

    /// Get the total size of guest memory in `self`'s memory
    /// layout.
    pub fn get_memory_size(&self) -> Result<usize> {
        let total_memory = self.code_size
            + Self::PAGE_TABLE_SIZE
            + self.sandbox_memory_config.host_function_definition_size
            + self.sandbox_memory_config.input_data_size
            + self.sandbox_memory_config.output_data_size
            + self.sandbox_memory_config.host_exception_size
            + self.sandbox_memory_config.guest_error_message_size
            + size_of::<HostFunctionDefinitions>()
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
            Err(anyhow!(
                "Total memory size {} exceeds limit of {}",
                size,
                Self::MAX_MEMORY_SIZE,
            ))
        } else {
            Ok(size)
        }
    }

    /// Write the finished memory layout to `guest_mem` and return
    /// `Ok` if successful.
    ///
    /// Note: `guest_mem` may have been modified, even if `Err` was returned
    /// from this function.
    pub fn write(
        &self,
        mut guest_mem: GuestMemory,
        guest_address: usize,
        size: usize,
    ) -> Result<()> {
        // Set up Guest Error Header
        guest_mem.write_usize(
            self.get_guest_error_offset(),
            self.guest_error_message_offset as u64,
        )?;

        guest_mem.write_usize(
            self.get_guest_error_message_pointer_offset(),
            self.get_guest_error_message_offset() as u64,
        )?;

        // Set up Host Exception Header
        guest_mem.write_usize(
            self.get_host_exception_size_offset(),
            self.sandbox_memory_config.host_exception_size as u64,
        )?;

        // Set up input buffer pointer
        guest_mem.write_usize(
            self.get_input_data_size_offset(),
            self.sandbox_memory_config.input_data_size as u64,
        )?;

        guest_mem.write_usize(
            self.get_input_data_pointer_offset(),
            self.get_input_data_offset() as u64,
        )?;

        // Set up output buffer pointer
        guest_mem.write_usize(
            self.get_output_data_size_offset(),
            self.sandbox_memory_config.output_data_size as u64,
        )?;
        guest_mem.write_usize(
            self.get_output_data_pointer_offset(),
            self.get_output_data_offset() as u64,
        )?;

        // Set up heap buffer pointer
        guest_mem.write_usize(self.get_heap_size_offset(), self.heap_size as u64)?;
        guest_mem.write_usize(
            self.get_heap_pointer_offset(),
            self.get_heap_offset() as u64,
        )?;

        // Set up Function Definition Header
        guest_mem.write_usize(
            self.get_function_definition_size_offset(),
            self.sandbox_memory_config.host_function_definition_size as u64,
        )?;
        guest_mem.write_usize(
            self.get_function_definition_pointer_offset(),
            self.get_function_definition_offset() as u64,
        )?;

        // Set up Max Guest Stack Address
        guest_mem.write_usize(
            self.get_min_guest_stack_address_offset(),
            (guest_address - (size - self.stack_size)) as u64,
        )?;
        Ok(())
    }
}
