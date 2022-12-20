use crate::mem::config::SandboxMemoryConfiguration;

/// Return a new `SandboxMemoryConfiguration` with the default
/// values filled in
#[no_mangle]
pub extern "C" fn mem_config_default() -> SandboxMemoryConfiguration {
    SandboxMemoryConfiguration::default()
}

/// Create a new SandboxMemoryConfiguration from the given
/// parameters.
#[no_mangle]
pub extern "C" fn mem_config_new(
    input_size: usize,
    output_size: usize,
    host_function_definition_size: usize,
    host_exception_size: usize,
    guest_error_message_size: usize,
) -> SandboxMemoryConfiguration {
    SandboxMemoryConfiguration::new(
        input_size,
        output_size,
        host_function_definition_size,
        host_exception_size,
        guest_error_message_size,
    )
}
