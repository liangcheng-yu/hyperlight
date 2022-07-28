use super::config::SandboxMemoryConfiguration;
use super::write_usize;
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
    pub peb_offset: usize,
    pub stack_size: usize,
    pub heap_size: usize,
    pub host_functions_offset: usize,
    pub host_exception_offset: usize,
    pub guest_error_message_offset: usize,
    pub code_and_outb_pointer_offset: usize,
    pub input_data_offset: usize,
    pub output_data_offset: usize,
    pub heap_data_offset: usize,
    pub stack_data_offset: usize,
    sandbox_memory_config: SandboxMemoryConfiguration,
    pub code_size: usize,
    pub host_functions_buffer_offset: usize,
    pub host_exception_buffer_offset: usize,
    pub guest_error_message_buffer_offset: usize,
    pub input_data_buffer_offset: usize,
    pub output_data_buffer_offset: usize,
    pub guest_heap_buffer_offset: usize,
    pub guest_stack_buffer_offset: usize,
    pub peb_address: usize,
}

impl SandboxMemoryLayout {
    /// Four Kilobytes (16^3 bytes) - used to round the total amount of memory
    /// used to the nearest 4K
    const FOUR_K: usize = 0x1000;

    /// The size of the page table within a sandbox
    const PAGE_TABLE_SIZE: usize = 0x3000;

    /// The offset into the sandbox's memory where the Page Directory starts.
    /// See https://www.pagetable.com/?p=14 for more information.
    const PD_OFFSET: usize = 0x2000;

    /// The offset into the sandbox's memory where the Page Directory Pointer
    /// Table is located.
    const PDPT_OFFSET: usize = 0x1000;

    /// The offset into the sandbox's memory where code starts.
    const CODE_OFFSET: usize = Self::PAGE_TABLE_SIZE;

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
        }
    }
    fn get_guest_error_message_address(&self, addr: usize) -> usize {
        addr + self.guest_error_message_buffer_offset
    }
    pub fn get_guest_error_address(&self, addr: usize) -> usize {
        addr + self.guest_error_message_offset
    }

    /// get the address of the size field in the guest error message buffer.
    /// this is the field after the `GuestErrorMessage` field, which is a `usize`
    fn get_guest_error_message_size_address(&self, addr: usize) -> usize {
        self.get_guest_error_address(addr) + size_of::<usize>()
    }

    /// pointer to the error message is after the Size field which is a ulong.
    pub fn get_guest_error_message_pointer_address(&self, addr: usize) -> usize {
        self.get_guest_error_message_size_address(addr) + size_of::<usize>()
    }

    pub fn get_function_definition_address(&self, addr: usize) -> usize {
        addr + self.host_functions_buffer_offset
    }

    fn get_function_definition_size_address(&self, addr: usize) -> usize {
        addr + self.host_functions_offset
    }

    /// Pointer to functions data is after the size field which is a ulong.
    fn get_function_definition_pointer_address(&self, addr: usize) -> usize {
        self.get_function_definition_size_address(addr) + size_of::<usize>()
    }

    fn get_host_exception_size_address(&self, addr: usize) -> usize {
        addr + self.host_exception_offset
    }

    pub fn get_host_exception_address(&self, addr: usize) -> usize {
        addr + self.host_exception_buffer_offset
    }

    /// OutB pointer is after the Code Pointer field which is a ulong..
    pub fn get_out_b_pointer_address(&self, addr: usize) -> usize {
        addr + size_of::<usize>()
    }

    fn get_output_data_size_address(&self, addr: usize) -> usize {
        addr + self.output_data_offset
    }

    /// Pointer to input data is after the size field which is a ulong.
    fn get_output_data_pointer_address(&self, addr: usize) -> usize {
        self.get_output_data_size_address(addr) + size_of::<usize>()
    }

    pub fn get_output_data_address(&self, addr: usize) -> usize {
        addr + self.output_data_buffer_offset
    }

    fn get_input_data_size_address(&self, addr: usize) -> usize {
        addr + self.input_data_offset
    }

    /// Pointer to input data is after the size field which is a ulong.
    fn get_input_data_pointer_address(&self, addr: usize) -> usize {
        self.get_input_data_size_address(addr) + size_of::<usize>()
    }

    pub fn get_input_data_address(&self, addr: usize) -> usize {
        addr + self.input_data_buffer_offset
    }
    pub fn get_code_pointer_address(self, addr: usize) -> usize {
        addr + self.code_and_outb_pointer_offset
    }

    /// Pointer to Dispatch Function is offset eight bytes into the FunctionDefinition.
    pub fn get_dispatch_function_pointer_address(&self, addr: usize) -> usize {
        self.get_function_definition_address(addr) + size_of::<usize>()
    }

    pub fn get_in_process_peb_address(&self, addr: usize) -> usize {
        addr + self.peb_offset
    }

    fn get_heap_size_address(&self, addr: usize) -> usize {
        addr + self.heap_data_offset
    }

    fn get_heap_pointer_address(&self, addr: usize) -> usize {
        self.get_heap_size_address(addr) + size_of::<usize>()
    }

    fn get_heap_address(&self, addr: usize) -> usize {
        addr + self.guest_heap_buffer_offset
    }

    fn get_min_guest_stack_address_pointer(&self, addr: usize) -> usize {
        addr + self.stack_data_offset
    }

    pub fn get_top_of_stack_address(&self, addr: usize) -> usize {
        addr + self.guest_stack_buffer_offset
    }

    pub fn get_host_pml4_address(addr: usize) -> usize {
        addr
    }

    pub fn get_host_pdpt_address(addr: usize) -> usize {
        addr + Self::PDPT_OFFSET
    }

    pub fn get_host_pd_address(addr: usize) -> usize {
        addr + Self::PD_OFFSET
    }

    pub fn get_host_code_address(addr: usize) -> usize {
        addr + Self::CODE_OFFSET
    }

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

    pub fn write_memory_layout(
        &self,
        guest_mem: &mut [u8],
        source_address: usize,
        guest_address: usize,
        size: usize,
    ) -> Result<()> {
        // Set up Guest Error Header
        write_usize(
            guest_mem,
            self.get_guest_error_address(source_address),
            self.guest_error_message_offset,
        )?;

        write_usize(
            guest_mem,
            self.get_guest_error_message_pointer_address(source_address),
            self.get_guest_error_message_address(guest_address),
        )?;

        // Set up Host Exception Header
        write_usize(
            guest_mem,
            self.get_host_exception_size_address(source_address),
            self.sandbox_memory_config.host_exception_size,
        )?;

        // Set up input buffer pointer
        write_usize(
            guest_mem,
            self.get_input_data_size_address(source_address),
            self.sandbox_memory_config.input_data_size,
        )?;

        write_usize(
            guest_mem,
            self.get_input_data_pointer_address(source_address),
            self.get_input_data_address(guest_address),
        )?;

        // Set up output buffer pointer
        write_usize(
            guest_mem,
            self.get_output_data_size_address(source_address),
            self.sandbox_memory_config.output_data_size,
        )?;
        write_usize(
            guest_mem,
            self.get_output_data_pointer_address(source_address),
            self.get_output_data_address(guest_address),
        )?;

        // Set up heap buffer pointer
        write_usize(
            guest_mem,
            self.get_heap_size_address(source_address),
            self.heap_size,
        )?;
        write_usize(
            guest_mem,
            self.get_heap_pointer_address(source_address),
            self.get_heap_address(guest_address),
        )?;

        // Set up Function Definition Header
        write_usize(
            guest_mem,
            self.get_function_definition_size_address(source_address),
            self.sandbox_memory_config.host_function_definition_size,
        )?;
        write_usize(
            guest_mem,
            self.get_function_definition_pointer_address(source_address),
            self.get_function_definition_address(guest_address),
        )?;

        // Set up Max Guest Stack Address
        write_usize(
            guest_mem,
            self.get_min_guest_stack_address_pointer(source_address),
            guest_address - (size - self.stack_size),
        )?;
        Ok(())
    }
}
