use crate::mem::config::SandboxMemoryConfiguration;

use super::option_when;

/// Return a new `SandboxMemoryConfiguration` with the default
/// values filled in
#[no_mangle]
pub extern "C" fn mem_config_default() -> SandboxMemoryConfiguration {
    SandboxMemoryConfiguration::default()
}

/// Create a new SandboxMemoryConfiguration from the given
/// parameters.
///
/// `stack_size_override` and `heap_size_override` are optional parameters
/// used to override the stack and heap sizes in the guest sandbox. if either
/// of these parameters are `0`, its value will be determined from the
/// guest binary's PE file header.
#[no_mangle]
pub extern "C" fn mem_config_new(
    input_size: usize,
    output_size: usize,
    host_function_definition_size: usize,
    host_exception_size: usize,
    guest_error_message_size: usize,
    stack_size_override: u64,
    heap_size_override: u64,
) -> SandboxMemoryConfiguration {
    SandboxMemoryConfiguration::new(
        input_size,
        output_size,
        host_function_definition_size,
        host_exception_size,
        guest_error_message_size,
        option_when(stack_size_override, stack_size_override == 0),
        option_when(heap_size_override, heap_size_override == 0),
    )
}
