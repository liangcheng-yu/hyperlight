use std::cmp::max;

/// The complete set of configuration needed to create a guest
/// memory region.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct SandboxMemoryConfiguration {
    /// The maximum size of the guest error message field.
    pub guest_error_message_size: usize,
    /// The size of the memory buffer that is made available for Guest Function Definitions
    pub host_function_definition_size: usize,
    /// The size of the memory buffer that is made available for serialising Host Exceptions
    pub host_exception_size: usize,
    /// The size of the memory buffer that is made available for input to the Guest Binary
    pub input_data_size: usize,
    /// The size of the memory buffer that is made available for input to the Guest Binary
    pub output_data_size: usize,
}

impl SandboxMemoryConfiguration {
    /// The default size of input data
    const DEFAULT_INPUT_SIZE: usize = 0x4000;
    const MIN_INPUT_SIZE: usize = 0x2000;
    /// The default size of output data
    const DEFAULT_OUTPUT_SIZE: usize = 0x4000;
    const MIN_OUTPUT_SIZE: usize = 0x2000;
    /// The default size of host function definitions
    const DEFAULT_HOST_FUNCTION_DEFINITION_SIZE: usize = 0x1000;
    const MIN_HOST_FUNCTION_DEFINITION_SIZE: usize = 0x400;
    /// The default size for host exceptions
    const DEFAULT_HOST_EXCEPTION_SIZE: usize = 0x4000;
    const MIN_HOST_EXCEPTION_SIZE: usize = 0x4000;
    /// The default size for guest error messages
    const DEFAULT_GUEST_ERROR_MESSAGE_SIZE: usize = 0x100;
    const MIN_GUEST_ERROR_MESSAGE_SIZE: usize = 0x80;

    /// Create a new configuration for a sandbox with the given sizes.
    pub fn new(
        input_data_size: usize,
        output_data_size: usize,
        function_definition_size: usize,
        host_exception_size: usize,
        guest_error_message_size: usize,
    ) -> Self {
        Self {
            input_data_size: max(input_data_size, Self::MIN_INPUT_SIZE),
            output_data_size: max(output_data_size, Self::MIN_OUTPUT_SIZE),
            host_function_definition_size: max(
                function_definition_size,
                Self::MIN_HOST_FUNCTION_DEFINITION_SIZE,
            ),
            host_exception_size: max(host_exception_size, Self::MIN_HOST_EXCEPTION_SIZE),
            guest_error_message_size: max(
                guest_error_message_size,
                Self::MIN_GUEST_ERROR_MESSAGE_SIZE,
            ),
        }
    }
}

impl Default for SandboxMemoryConfiguration {
    fn default() -> Self {
        Self {
            guest_error_message_size: Self::DEFAULT_GUEST_ERROR_MESSAGE_SIZE,
            host_function_definition_size: Self::DEFAULT_HOST_FUNCTION_DEFINITION_SIZE,
            host_exception_size: Self::DEFAULT_HOST_EXCEPTION_SIZE,
            input_data_size: Self::DEFAULT_INPUT_SIZE,
            output_data_size: Self::DEFAULT_OUTPUT_SIZE,
        }
    }
}
