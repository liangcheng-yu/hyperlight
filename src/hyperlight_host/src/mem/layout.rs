use super::{config::SandboxMemoryConfiguration, guest_mem::GuestMemory};
use anyhow::{anyhow, Result};
use paste::paste;
use rand::rngs::OsRng;
use rand::RngCore;
use std::mem::size_of;

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
    guest_error_code: u64,
    max_message_size: u64,
    message: usize,
}

#[repr(C)]
struct CodeAndOutBPointers {
    code_pointer: u64,
    outb_pointer: u64,
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
// Host Function definitions - length sandboxMemoryConfiguration.HostFunctionDefinitionSize
// Host Exception Details - length sandboxMemoryConfiguration.HostExceptionSize , this contains details of any Host Exception that occurred in outb function
// it contains a 32 bit length following by a json serialisation of any error that occurred.
// Guest Error Details - length sandboxMemoryConfiguration.GuestErrorMessageSize this contains an error message string for any guest error that occurred.
// Input Data Buffer - length sandboxMemoryConfiguration.InputDataSize this is a buffer that is used for input data to the host program
// Output Data Buffer - length sandboxMemoryConfiguration.OutputDataSize this is a buffer that is used for output data from host program
// Guest Heap - length heapSize this is a memory buffer provided to the guest to be used as heap memory.
// Guest Stack - length stackSize this is the memory used for the guest stack (in reality the stack may be slightly bigger or smaller as the total memory size is rounded up to nearest 4K and there is a 16 bte stack guard written to the top of the stack).

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
///
///

#[derive(Copy, Clone, Debug)]
pub struct SandboxMemoryLayout {
    sandbox_memory_config: SandboxMemoryConfiguration,
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
    pub guest_error_offset: usize,
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
    /// The size of code inside this sandbox.
    pub code_size: usize,
    /// The offset to the start of the definitions of host functions inside
    /// this sandbox.
    pub host_function_definitions_offset: usize,
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
    pub const PAGE_TABLE_SIZE: usize = 0x3000;
    /// The offset into the sandbox's memory where the PML4 Table is located.
    /// See https://www.pagetable.com/?p=14 for more information.
    pub const PML4_OFFSET: usize = 0x0000;
    /// The offset into the sandbox's memory where the Page Directory starts.
    pub const PD_OFFSET: usize = 0x2000;
    /// The offset into the sandbox's memory where the Page Directory Pointer Table is located.
    pub const PDPT_OFFSET: usize = 0x1000;
    /// The offset into the sandbox's memory where code starts.
    pub const CODE_OFFSET: usize = Self::PAGE_TABLE_SIZE;
    /// The maximum amount of memory a single sandbox will be allowed.
    const MAX_MEMORY_SIZE: usize = 0x3FEF0000;

    /// The base address of the sandbox's memory.
    pub const BASE_ADDRESS: usize = 0x0200000;

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
        let peb_offset = Self::PAGE_TABLE_SIZE + code_size;
        let guest_security_cookie_seed_offset = Self::PAGE_TABLE_SIZE + code_size;
        let host_functions_offset =
            guest_security_cookie_seed_offset + size_of::<GuestSecurityCookie>();
        let host_exception_offset = host_functions_offset + size_of::<HostFunctions>();
        let guest_error_message_offset = host_exception_offset + size_of::<HostExceptionData>();
        let code_and_outb_pointer_offset = guest_error_message_offset + size_of::<GuestError>();
        let input_data_offset = code_and_outb_pointer_offset + size_of::<CodeAndOutBPointers>();
        let output_data_offset = input_data_offset + size_of::<InputData>();
        let heap_data_offset = output_data_offset + size_of::<OutputData>();
        let stack_data_offset = heap_data_offset + size_of::<GuestHeap>();
        let peb_address = Self::BASE_ADDRESS + peb_offset;
        let host_function_definitions_offset = stack_data_offset + size_of::<GuestStack>();
        let host_exception_buffer_offset =
            host_function_definitions_offset + cfg.host_function_definition_size;
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
            guest_error_offset: guest_error_message_offset,
            code_and_outb_pointer_offset,
            input_data_offset,
            output_data_offset,
            heap_data_offset,
            stack_data_offset,
            sandbox_memory_config: cfg,
            code_size,
            host_function_definitions_offset,
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

    /// Get the offset in guest memory to the size field in the
    /// host functions structure.
    pub fn get_host_exception_size_offset(&self) -> usize {
        // The size field is the first field in the `HostFunctions` struct
        self.host_exception_offset
    }

    /// Get the offset in guest memory to the size field in the
    /// guest error message structure.
    pub fn get_guest_error_message_size_offset(&self) -> usize {
        // This is the field after the `GuestErrorMessage` field,
        // which is a `u64`
        self.guest_error_offset + size_of::<u64>()
    }

    /// Get the offset in guest memory to the error message pointer in the
    /// guest error message structure.
    pub fn get_guest_error_message_pointer_offset(&self) -> usize {
        // This offset is after the message size, which is a `u64`.
        self.get_guest_error_message_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the output data size
    pub fn get_output_data_size_offset(&self) -> usize {
        // The size field is the first field in the `OutputData` struct
        self.output_data_offset
    }

    /// Get the offset in guest memory to the host function definitions
    /// size
    pub fn get_host_function_definitions_size_offset(&self) -> usize {
        // The size field is the first field in the `HostFunctions` struct
        self.host_functions_offset
    }

    /// Get the offset in guest memory to the host function definitions
    /// pointer.
    pub fn get_host_function_definitions_pointer_offset(&self) -> usize {
        // The size field is the field after the size field in the `HostFunctions` struct which is a u64
        self.get_host_function_definitions_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the minimum guest stack address.
    pub fn get_min_guest_stack_address_offset(&self) -> usize {
        // The minimum guest stack address is the start of the guest stack
        self.stack_data_offset
    }

    /// Get the offset in guest memory to the start of host exceptions
    pub fn get_host_exception_offset(&self) -> usize {
        self.host_exception_buffer_offset
    }

    /// Get the offset in guest memory to the OutB pointer.
    pub fn get_out_b_pointer_offset(&self) -> usize {
        // The outb pointer is immediately after the code pointer
        // in the `CodeAndOutBPointers` struct which is a u64
        self.code_and_outb_pointer_offset + size_of::<u64>()
    }

    /// Get the offset in guest memory to the output data pointer.
    pub fn get_output_data_pointer_offset(&self) -> usize {
        // This field is immedaitely after the output data size field,
        // which is a `u64`.
        self.get_output_data_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the start of output data.
    pub fn get_output_data_offset(&self) -> usize {
        self.output_data_buffer_offset
    }

    /// Get the offset in guest memory to the input data size.
    pub fn get_input_data_size_offset(&self) -> usize {
        // The input data size is the first field in the `InputData` struct
        self.input_data_offset
    }

    /// Get the offset in guest memory to the input data pointer.
    pub fn get_input_data_pointer_offset(&self) -> usize {
        // The input data pointer is immediately after the input
        // data size field in the `InputData` struct which is a `u64`.
        self.get_input_data_size_offset() + size_of::<u64>()
    }

    /// Get the offset in guest memory to the code pointer
    pub fn get_code_pointer_offset(&self) -> usize {
        // The code pointer is the first field
        // in the `CodeAndOutBPointers` struct which is a u64
        self.code_and_outb_pointer_offset
    }

    /// Get the offset in guest memory to the dispatch function
    /// pointer.
    pub fn get_dispatch_function_pointer_offset(&self) -> usize {
        // The dispatch function pointer is the field aftter the count of functions
        // in the host function definitions struct which is a u64
        self.host_function_definitions_offset + size_of::<u64>()
    }

    /// Get the offset in guest memory to the PEB address
    pub fn get_in_process_peb_offset(&self) -> usize {
        self.peb_offset
    }

    /// Get the offset in guest memory to the heap size
    pub fn get_heap_size_offset(&self) -> usize {
        self.heap_data_offset
    }

    /// Get the offset of the heap pointer in guest memory,
    pub fn get_heap_pointer_offset(&self) -> usize {
        // The heap pointer is immediately after the
        // heap size field in the `GuestHeap` struct which is a `u64`.
        self.get_heap_size_offset() + size_of::<u64>()
    }

    /// Get the offset to the top of the stack in guest memory
    pub fn get_top_of_stack_offset(&self) -> usize {
        self.guest_stack_buffer_offset
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
        guest_mem: &mut GuestMemory,
        guest_offset: usize,
        size: usize,
    ) -> Result<()> {
        macro_rules! get_address {
            ($something:ident) => {
                paste! {
                    if guest_offset == 0 {
                        guest_mem.calculate_address(self.[<$something _offset>])?
                    } else {
                        guest_offset +  self.[<$something _offset>]
                    } as u64
                }
            };
        }

        if guest_offset != SandboxMemoryLayout::BASE_ADDRESS
            && guest_offset != guest_mem.base_addr()
        {
            return Err(anyhow!(
                "Guest offset {} is not a valid guest offset",
                guest_offset
            ));
        }

        // Set up Guest Error Header
        guest_mem.write_u64(
            self.get_guest_error_message_size_offset(),
            self.sandbox_memory_config.guest_error_message_size as u64,
        )?;

        let addr = get_address!(guest_error_message_buffer);

        guest_mem.write_u64(self.get_guest_error_message_pointer_offset(), addr)?;

        // Set up Host Exception Header
        guest_mem.write_u64(
            self.get_host_exception_size_offset(),
            self.sandbox_memory_config.host_exception_size as u64,
        )?;

        // Set up input buffer pointer
        guest_mem.write_u64(
            self.get_input_data_size_offset(),
            self.sandbox_memory_config.input_data_size as u64,
        )?;

        let addr = get_address!(input_data_buffer);

        guest_mem.write_u64(self.get_input_data_pointer_offset(), addr)?;

        // Set up output buffer pointer
        guest_mem.write_u64(
            self.get_output_data_size_offset(),
            self.sandbox_memory_config.output_data_size as u64,
        )?;

        let addr = get_address!(output_data_buffer);

        guest_mem.write_u64(self.get_output_data_pointer_offset(), addr)?;

        let addr = get_address!(guest_heap_buffer);

        // Set up heap buffer pointer
        guest_mem.write_u64(self.get_heap_size_offset(), self.heap_size as u64)?;
        guest_mem.write_u64(self.get_heap_pointer_offset(), addr)?;

        let addr = get_address!(host_function_definitions);

        // Set up Host Function Definition
        guest_mem.write_u64(
            self.get_host_function_definitions_size_offset(),
            self.sandbox_memory_config.host_function_definition_size as u64,
        )?;
        guest_mem.write_u64(self.get_host_function_definitions_pointer_offset(), addr)?;

        // Set up Min Guest Stack Address
        guest_mem.write_u64(
            self.get_min_guest_stack_address_offset(),
            (guest_offset + (size - self.stack_size)) as u64,
        )?;

        // Set up the security cookie seed

        let mut security_cookie_seed = [0u8; 8];
        OsRng.fill_bytes(&mut security_cookie_seed);

        guest_mem.copy_into(
            &security_cookie_seed,
            self.guest_security_cookie_seed_offset,
        )?;

        Ok(())
    }
}
