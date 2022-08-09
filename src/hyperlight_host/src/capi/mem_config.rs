use super::context::{Context, ReadResult, WriteResult};
use super::handle::Handle;
use super::hdl::Hdl;
use crate::mem::config::SandboxMemoryConfiguration;

/// Get a read-only reference to a `SandboxMemoryConfiguration` within
/// `ctx` that is referenced by `handle`.
pub fn get_mem_config(ctx: &Context, handle: Handle) -> ReadResult<SandboxMemoryConfiguration> {
    Context::get(handle, &ctx.mem_configs, |m| matches!(m, Hdl::MemConfig(_)))
}

/// get a `SandboxMemoryConfiguration` wrapped in a `WriteResult`,
/// which makes it suitable for overwriting in a concurrency-safe
/// manner.
pub fn get_mem_config_mut(
    ctx: &Context,
    handle: Handle,
) -> WriteResult<SandboxMemoryConfiguration> {
    Context::get_mut(handle, &ctx.mem_configs, |m| matches!(m, Hdl::MemConfig(_)))
}

/// Create a new sandbox memory configuration within `ctx`
/// with the given parameters.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_new(
    ctx: *mut Context,
    input_data_size: usize,
    output_data_size: usize,
    function_definition_size: usize,
    host_exception_size: usize,
    guest_error_message_size: usize,
) -> Handle {
    let config = SandboxMemoryConfiguration::new(
        input_data_size,
        output_data_size,
        function_definition_size,
        host_exception_size,
        guest_error_message_size,
    );

    Context::register(config, &(*ctx).mem_configs, Hdl::MemConfig)
}

/// Get the guest error message size from the memory configuration
/// referenced by the given `Handle`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_get_guest_error_message_size(
    ctx: *const Context,
    hdl: Handle,
) -> usize {
    match get_mem_config(&*ctx, hdl) {
        Ok(c) => c.guest_error_message_size,
        Err(_) => 0,
    }
}

/// Fetch the memory configuration referenced by `hdl` inside `ctx`,
/// set its `host_function_definition_size` field to `val`,
/// and return the value that was previously set.
///
/// Returns 0 if `hdl` does not reference a valid memory configuration
/// within `ctx`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_set_guest_error_message_size(
    ctx: *const Context,
    hdl: Handle,
    val: usize,
) -> usize {
    match get_mem_config_mut(&*ctx, hdl) {
        Ok(mut c) => {
            let old = c.guest_error_message_size;
            c.guest_error_message_size = val;
            old
        }
        Err(_) => 0,
    }
}

/// Get the host function definition size from the memory configuration
/// referenced by the given `Handle`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_get_host_function_definition_size(
    ctx: *const Context,
    hdl: Handle,
) -> usize {
    match get_mem_config(&*ctx, hdl) {
        Ok(c) => c.host_function_definition_size,
        Err(_) => 0,
    }
}

/// Fetch the memory configuration referenced by `hdl` inside `ctx`,
/// set its `host_function_definition_size` field to `val`,
/// and return the value that was previously set.
///
/// Returns 0 if `hdl` does not reference a valid memory configuration
/// within `ctx`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_set_host_function_definition_size(
    ctx: *const Context,
    hdl: Handle,
    val: usize,
) -> usize {
    match get_mem_config_mut(&*ctx, hdl) {
        Ok(mut c) => {
            let old = c.host_function_definition_size;
            c.host_function_definition_size = val;
            old
        }
        Err(_) => 0,
    }
}

/// Get the host exception size from the memory configuration
/// referenced by the given `Handle`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_get_host_exception_size(
    ctx: *const Context,
    hdl: Handle,
) -> usize {
    match get_mem_config(&*ctx, hdl) {
        Ok(c) => c.host_exception_size,
        Err(_) => 0,
    }
}

/// Fetch the memory configuration referenced by `hdl` inside `ctx`,
/// set its `host_exception_size` field to `val`, and return the value
/// that was previously set.
///
/// Returns 0 if `hdl` does not reference a valid memory configuration
/// within `ctx`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_set_host_exception_size(
    ctx: *const Context,
    hdl: Handle,
    val: usize,
) -> usize {
    match get_mem_config_mut(&*ctx, hdl) {
        Ok(mut c) => {
            let old = c.host_exception_size;
            c.host_exception_size = val;
            old
        }
        Err(_) => 0,
    }
}

/// Get the input data size from the memory configuration
/// referenced by the given `Handle`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_get_input_data_size(ctx: *const Context, hdl: Handle) -> usize {
    match get_mem_config(&*ctx, hdl) {
        Ok(c) => c.input_data_size,
        Err(_) => 0,
    }
}

/// Fetch the memory configuration referenced by `hdl` inside `ctx`,
/// set its `input_data_size` field to `val`, and return the value
/// that was previously set.
///
/// Returns 0 if `hdl` does not reference a valid memory configuration
/// within `ctx`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_set_input_data_size(
    ctx: *const Context,
    hdl: Handle,
    val: usize,
) -> usize {
    match get_mem_config_mut(&*ctx, hdl) {
        Ok(mut c) => {
            let old = c.input_data_size;
            c.input_data_size = val;
            old
        }
        Err(_) => 0,
    }
}

/// Get the output data size from the memory configuration
/// referenced by the given `Handle`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_get_output_data_size(
    ctx: *const Context,
    hdl: Handle,
) -> usize {
    match get_mem_config(&*ctx, hdl) {
        Ok(c) => c.output_data_size,
        Err(_) => 0,
    }
}

/// Fetch the memory configuration referenced by `hdl` inside `ctx`,
/// set its `output_data_size` field to `val`, and return the value
/// that was previously set.
///
/// Returns 0 if `hdl` does not reference a valid memory configuration
/// within `ctx`.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_config_set_output_data_size(
    ctx: *const Context,
    hdl: Handle,
    val: usize,
) -> usize {
    match get_mem_config_mut(&*ctx, hdl) {
        Ok(mut c) => {
            let old = c.output_data_size;
            c.output_data_size = val;
            old
        }
        Err(_) => 0,
    }
}
