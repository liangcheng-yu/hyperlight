use std::time::Duration;

use hyperlight_host::sandbox::SandboxConfiguration;

/// Return a new `SandboxConfiguration` with the default
/// values filled in
#[no_mangle]
pub extern "C" fn config_default() -> SandboxConfiguration {
    SandboxConfiguration::default()
}

/// Create a new SandboxConfiguration from the given
/// parameters.
///
/// `stack_size_override` and `heap_size_override` are optional parameters
/// used to override the stack and heap sizes in the guest sandbox. if either
/// of these parameters are `0`, its value will be determined from the
/// guest binary's PE file header.
#[no_mangle]
pub extern "C" fn config_new(
    input_size: usize,
    output_size: usize,
    host_function_definition_size: usize,
    host_exception_size: usize,
    guest_error_message_size: usize,
    stack_size_override: u64,
    heap_size_override: u64,
    kernel_stack_size: usize,
    max_execution_time: u16,
    max_wait_for_cancellation: u8,
) -> SandboxConfiguration {
    let mut config = SandboxConfiguration::default();
    config.set_input_data_size(input_size);
    config.set_output_data_size(output_size);
    config.set_host_function_definition_size(host_function_definition_size);
    config.set_host_exception_size(host_exception_size);
    config.set_guest_error_buffer_size(guest_error_message_size);
    config.set_stack_size(stack_size_override);
    config.set_heap_size(heap_size_override);
    config.set_kernel_stack_size(kernel_stack_size);
    config.set_max_execution_time(Duration::from_millis(max_execution_time as u64));
    config.set_max_execution_cancel_wait_time(Duration::from_millis(
        max_wait_for_cancellation as u64,
    ));
    config
}
