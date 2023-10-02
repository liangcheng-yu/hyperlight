use std::{cmp::max, time::Duration};

use crate::option_when;

use crate::mem::pe::pe_info::PEInfo;

/// The complete set of configuration needed to create a guest
/// memory region.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct SandboxConfiguration {
    /// The maximum size of the guest error buffer.
    pub guest_error_buffer_size: usize,
    /// The size of the memory buffer that is made available for Guest Function
    /// Definitions
    pub host_function_definition_size: usize,
    /// The size of the memory buffer that is made available for serialising
    /// Host Exceptions
    pub host_exception_size: usize,
    /// The size of the memory buffer that is made available for input to the
    /// Guest Binary
    pub input_data_size: usize,
    /// The size of the memory buffer that is made available for input to the
    /// Guest Binary
    pub output_data_size: usize,
    /// The stack size to use in the guest sandbox. If set to 0, the stack
    /// size will be determined from the PE file header
    ///
    /// Note: this is a C-compatible struct, so even though this optional
    /// field should be represented as an `Option`, that type is not
    /// FFI-safe, so it cannot be.
    pub stack_size_override: u64,
    /// The heap size to use in the guest sandbox. If set to 0, the heap
    /// size will be determined from the PE file header
    ///
    /// Note: this is a C-compatible struct, so even though this optional
    /// field should be represented as an `Option`, that type is not
    /// FFI-safe, so it cannot be.
    pub heap_size_override: u64,
    /// The max_execution_time of a guest execution in milliseconds. If set to 0, the max_execution_time
    /// will be set to the default value of 1000ms if the guest execution does not complete within the time specified
    /// then the execution will be cancelled, the minimum value is 1ms
    ///
    /// Note: this is a C-compatible struct, so even though this optional
    /// field should be represented as an `Option`, that type is not
    /// FFI-safe, so it cannot be.
    pub max_execution_time: u16,
    /// The max_wait_for_cancellation represents the maximum time the host should wait for a guest execution to be cancelled
    /// If set to 0, the max_wait_for_cancellation will be set to the default value of 10ms.
    /// The minimum value is 1ms.
    ///
    /// Note: this is a C-compatible struct, so even though this optional
    /// field should be represented as an `Option`, that type is not
    /// FFI-safe, so it cannot be.
    pub max_wait_for_cancellation: u8,
}

impl SandboxConfiguration {
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
    const DEFAULT_GUEST_ERROR_BUFFER_SIZE: usize = 0x100;
    const MIN_GUEST_ERROR_BUFFER_SIZE: usize = 0x80;
    /// The default and minimum values for max execution time (in milliseconds)
    const DEFAULT_MAX_EXECUTION_TIME: u16 = 1000;
    const MIN_MAX_EXECUTION_TIME: u16 = 1;
    /// The default and minimum values for max wait for cancellation (in milliseconds)
    const DEFAULT_MAX_WAIT_FOR_CANCELLATION: u8 = 100;
    const MIN_MAX_WAIT_FOR_CANCELLATION: u8 = 10;

    #[allow(clippy::too_many_arguments)]
    /// Create a new configuration for a sandbox with the given sizes.
    pub fn new(
        input_data_size: usize,
        output_data_size: usize,
        function_definition_size: usize,
        host_exception_size: usize,
        guest_error_buffer_size: usize,
        stack_size_override: Option<u64>,
        heap_size_override: Option<u64>,
        max_execution_time: Option<Duration>,
        max_wait_for_cancellation: Option<Duration>,
    ) -> Self {
        Self {
            input_data_size: max(input_data_size, Self::MIN_INPUT_SIZE),
            output_data_size: max(output_data_size, Self::MIN_OUTPUT_SIZE),
            host_function_definition_size: max(
                function_definition_size,
                Self::MIN_HOST_FUNCTION_DEFINITION_SIZE,
            ),
            host_exception_size: max(host_exception_size, Self::MIN_HOST_EXCEPTION_SIZE),
            guest_error_buffer_size: max(
                guest_error_buffer_size,
                Self::MIN_GUEST_ERROR_BUFFER_SIZE,
            ),
            stack_size_override: stack_size_override.unwrap_or(0),
            heap_size_override: heap_size_override.unwrap_or(0),
            max_execution_time: {
                match max_execution_time {
                    Some(max_execution_time) => match max_execution_time.as_millis() {
                        0 => Self::DEFAULT_MAX_EXECUTION_TIME,
                        1..=65_535u128 => max(
                            max_execution_time.as_millis(),
                            Self::MIN_MAX_EXECUTION_TIME.into(),
                        ) as u16,
                        _ => Self::DEFAULT_MAX_EXECUTION_TIME,
                    },
                    None => Self::DEFAULT_MAX_EXECUTION_TIME,
                }
            },
            max_wait_for_cancellation: {
                match max_wait_for_cancellation {
                    Some(max_wait_for_cancellation) => {
                        match max_wait_for_cancellation.as_millis() {
                            0 => Self::DEFAULT_MAX_WAIT_FOR_CANCELLATION,
                            1..=255u128 => max(
                                max_wait_for_cancellation.as_millis(),
                                Self::MIN_MAX_WAIT_FOR_CANCELLATION.into(),
                            ) as u8,
                            _ => Self::DEFAULT_MAX_WAIT_FOR_CANCELLATION,
                        }
                    }
                    None => Self::DEFAULT_MAX_WAIT_FOR_CANCELLATION,
                }
            },
        }
    }

    fn stack_size_override_opt(&self) -> Option<u64> {
        option_when(self.stack_size_override, self.stack_size_override > 0)
    }

    fn heap_size_override_opt(&self) -> Option<u64> {
        option_when(self.heap_size_override, self.heap_size_override > 0)
    }

    /// If self.stack_size is non-zero, return it. Otherwise,
    /// return pe_info.stack_reserve()
    pub(crate) fn get_stack_size(&self, pe_info: &PEInfo) -> u64 {
        self.stack_size_override_opt()
            .unwrap_or_else(|| pe_info.stack_reserve())
    }

    /// If self.heap_size_override is non-zero, return it. Otherwise,
    /// return pe_info.heap_reserve()
    pub(crate) fn get_heap_size(&self, pe_info: &PEInfo) -> u64 {
        self.heap_size_override_opt()
            .unwrap_or_else(|| pe_info.heap_reserve())
    }
}

impl Default for SandboxConfiguration {
    fn default() -> Self {
        Self::new(
            Self::DEFAULT_INPUT_SIZE,
            Self::DEFAULT_OUTPUT_SIZE,
            Self::DEFAULT_HOST_FUNCTION_DEFINITION_SIZE,
            Self::DEFAULT_HOST_EXCEPTION_SIZE,
            Self::DEFAULT_GUEST_ERROR_BUFFER_SIZE,
            None,
            None,
            None,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::SandboxConfiguration;
    use std::time::Duration;

    use crate::testing::{callback_guest_pe_info, simple_guest_pe_info};

    #[test]
    fn overrides() {
        const STACK_SIZE_OVERRIDE: u64 = 0x10000;
        const HEAP_SIZE_OVERRIDE: u64 = 0x50000;
        const INPUT_DATA_SIZE_OVERRIDE: usize = 0x4000;
        const OUTPUT_DATA_SIZE_OVERRIDE: usize = 0x4001;
        const HOST_FUNCTION_DEFINITION_SIZE_OVERRIDE: usize = 0x4002;
        const HOST_EXCEPTION_SIZE_OVERRIDE: usize = 0x4003;
        const GUEST_ERROR_BUFFER_SIZE_OVERRIDE: usize = 0x40004;
        const MAX_EXECUTION_TIME_OVERRIDE: u16 = 1010;
        const MAX_WAIT_FOR_CANCELLATION_OVERRIDE: u8 = 200;
        let cfg = SandboxConfiguration::new(
            INPUT_DATA_SIZE_OVERRIDE,
            OUTPUT_DATA_SIZE_OVERRIDE,
            HOST_FUNCTION_DEFINITION_SIZE_OVERRIDE,
            HOST_EXCEPTION_SIZE_OVERRIDE,
            GUEST_ERROR_BUFFER_SIZE_OVERRIDE,
            Some(STACK_SIZE_OVERRIDE),
            Some(HEAP_SIZE_OVERRIDE),
            Some(Duration::from_millis(MAX_EXECUTION_TIME_OVERRIDE as u64)),
            Some(Duration::from_millis(
                MAX_WAIT_FOR_CANCELLATION_OVERRIDE as u64,
            )),
        );
        let pe_infos = vec![
            simple_guest_pe_info().unwrap(),
            callback_guest_pe_info().unwrap(),
        ];
        for pe_info in pe_infos {
            let stack_size = cfg.get_stack_size(&pe_info);
            let heap_size = cfg.get_heap_size(&pe_info);
            assert_eq!(STACK_SIZE_OVERRIDE, stack_size);
            assert_eq!(HEAP_SIZE_OVERRIDE, heap_size);
        }
        let cfg = SandboxConfiguration::new(
            INPUT_DATA_SIZE_OVERRIDE,
            OUTPUT_DATA_SIZE_OVERRIDE,
            HOST_FUNCTION_DEFINITION_SIZE_OVERRIDE,
            HOST_EXCEPTION_SIZE_OVERRIDE,
            GUEST_ERROR_BUFFER_SIZE_OVERRIDE,
            Some(1024),
            Some(2048),
            Some(Duration::from_millis(MAX_EXECUTION_TIME_OVERRIDE as u64)),
            Some(Duration::from_millis(
                MAX_WAIT_FOR_CANCELLATION_OVERRIDE as u64,
            )),
        );
        assert_eq!(1024, cfg.stack_size_override);
        assert_eq!(2048, cfg.heap_size_override);
        assert_eq!(INPUT_DATA_SIZE_OVERRIDE, cfg.input_data_size);
        assert_eq!(OUTPUT_DATA_SIZE_OVERRIDE, cfg.output_data_size);
        assert_eq!(
            HOST_FUNCTION_DEFINITION_SIZE_OVERRIDE,
            cfg.host_function_definition_size
        );
        assert_eq!(HOST_EXCEPTION_SIZE_OVERRIDE, cfg.host_exception_size);
        assert_eq!(
            GUEST_ERROR_BUFFER_SIZE_OVERRIDE,
            cfg.guest_error_buffer_size
        );
        assert_eq!(MAX_EXECUTION_TIME_OVERRIDE, cfg.max_execution_time);
        assert_eq!(
            MAX_WAIT_FOR_CANCELLATION_OVERRIDE,
            cfg.max_wait_for_cancellation
        );
    }
}
