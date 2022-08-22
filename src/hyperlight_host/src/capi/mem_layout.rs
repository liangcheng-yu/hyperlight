use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::Addr;
use crate::mem::layout::SandboxMemoryLayout;
use anyhow::{anyhow, Result};

mod impls {
    use crate::capi::context::Context;
    use crate::capi::guest_mem::get_guest_memory;
    use crate::capi::handle::Handle;
    use crate::capi::mem_config::get_mem_config;
    use crate::capi::Addr;
    use crate::mem::layout::SandboxMemoryLayout;
    use anyhow::Result;

    pub fn new(
        ctx: &mut Context,
        mem_cfg_ref: Handle,
        code_size: usize,
        stack_size: usize,
        heap_size: usize,
    ) -> Result<SandboxMemoryLayout> {
        let cfg = get_mem_config(ctx, mem_cfg_ref)?;
        Ok(SandboxMemoryLayout::new(
            *cfg, code_size, stack_size, heap_size,
        ))
    }

    /// Fetch the memory layout in `ctx` referenced by `layout_ref`,
    /// then convert call `fetcher_fn` with that layout and add the
    /// result with `base`, and finally return the result of that
    /// addition.
    pub fn calculate_address<F: FnOnce(&SandboxMemoryLayout) -> Result<usize>>(
        ctx: &Context,
        layout_ref: Handle,
        base: i64,
        fetcher_fn: F,
    ) -> Result<i64> {
        let layout = super::get_mem_layout(ctx, layout_ref)?;
        let base = Addr::from_i64(base)?;
        let offset = fetcher_fn(&layout)?;
        base.add_usize(offset).as_i64()
    }

    pub fn get_memory_size(ctx: &Context, mem_layout_ref: Handle) -> Result<usize> {
        let layout = super::get_mem_layout(ctx, mem_layout_ref)?;
        layout.get_memory_size()
    }

    pub fn write_memory_layout(
        ctx: &Context,
        mem_layout_ref: Handle,
        guest_mem_ref: Handle,
        guest_address: usize,
        size: usize,
    ) -> Result<()> {
        let layout = super::get_mem_layout(ctx, mem_layout_ref)?;
        let guest_mem = get_guest_memory(ctx, guest_mem_ref)?;
        layout.write(*guest_mem, guest_address, size)
    }
}

/// Get the `SandboxMemoryLayout` stored in `ctx` and referenced
/// by `handle`, or `Err` if no such layout exists.
pub fn get_mem_layout(ctx: &Context, handle: Handle) -> Result<SandboxMemoryLayout> {
    let res = Context::get(handle, &ctx.mem_layouts, |h| matches!(h, Hdl::MemLayout(_)));
    match res {
        Ok(ml) => Ok(*ml),
        Err(e) => Err(anyhow!(e)),
    }
}

/// Create a new memory layout within `ctx` with the given parameters and return
/// a reference to it.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_layout_new(
    ctx: *mut Context,
    mem_cfg_ref: Handle,
    code_size: usize,
    stack_size: usize,
    heap_size: usize,
) -> Handle {
    match impls::new(&mut *ctx, mem_cfg_ref, code_size, stack_size, heap_size) {
        Ok(layout) => Context::register(layout, &(*ctx).mem_layouts, Hdl::MemLayout),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get the stack size from the memory layout in `ctx` referenced
/// by `mem_layout_ref`, or `0` if no such memory layout exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_stack_size(
    ctx: *const Context,
    mem_layout_ref: Handle,
) -> usize {
    get_mem_layout(&*ctx, mem_layout_ref)
        .map(|ml| ml.stack_size)
        .unwrap_or(0)
}

/// Get the heap size from the memory layout in `ctx` referenced
/// by `mem_layout_ref`, or `0` if no such memory layout exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_heap_size(
    ctx: *const Context,
    mem_layout_ref: Handle,
) -> usize {
    get_mem_layout(&*ctx, mem_layout_ref)
        .map(|ml| ml.heap_size)
        .unwrap_or(0)
}

/// Get the host functions offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_host_functions_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.host_functions_offset)
    })
    .unwrap_or(0)
}

/// Get the guest error message offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_guest_error_message_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.guest_error_message_offset)
    })
    .unwrap_or(0)
}
/// Get the code and outb pointer offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_code_and_outb_pointer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.code_and_outb_pointer_offset)
    })
    .unwrap_or(0)
}

/// Get the output data offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_output_data_offset(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.output_data_offset)
    })
    .unwrap_or(0)
}
/// Get the heap data offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_heap_data_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| Ok(l.heap_data_offset))
        .unwrap_or(0)
}
/// Get the stack data offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_stack_data_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(
        &*ctx,
        mem_layout_ref,
        base_addr,
        |l| Ok(l.stack_data_offset),
    )
    .unwrap_or(0)
}

/// Get the code size from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_code_size(
    ctx: *const Context,
    mem_layout_ref: Handle,
) -> usize {
    match get_mem_layout(&*ctx, mem_layout_ref) {
        Ok(ml) => ml.code_size,
        Err(_) => 0,
    }
}

/// Using the memory layout `mem_layout_ref` in `ctx`, get the
/// address to the host functions buffer, given `base_addr` as the
/// base address, or `0` if no such memory layout exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_host_functions_buffer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.host_functions_buffer_offset)
    })
    .unwrap_or(0)
}

/// Get the host exception buffer offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_host_exception_buffer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.host_exception_buffer_offset)
    })
    .unwrap_or(0)
}

/// Get the guest security seed offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_guest_security_cookie_seed_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.guest_security_cookie_seed_offset)
    })
    .unwrap_or(0)
}

/// Get the guest error message buffer offset from the memory layout in
/// `ctx` referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_guest_error_message_buffer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.guest_error_message_buffer_offset)
    })
    .unwrap_or(0)
}

/// Get the input data buffer offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_input_data_buffer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.input_data_buffer_offset)
    })
    .unwrap_or(0)
}

/// Get the output data buffer offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_output_data_buffer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.output_data_buffer_offset)
    })
    .unwrap_or(0)
}

/// Get the heap buffer offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_guest_heap_buffer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.guest_heap_buffer_offset)
    })
    .unwrap_or(0)
}

/// Get the guest stack buffer offset from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_guest_stack_buffer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.guest_stack_buffer_offset)
    })
    .unwrap_or(0)
}
/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_peb_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
) -> i64 {
    // NOTE: using calculate_address here for the safe conversions
    // from usize -> i64, but not for calculating offsets, as we
    // do in most of the other functions herein.
    impls::calculate_address(&*ctx, mem_layout_ref, 0, |l| Ok(l.peb_address)).unwrap_or(0)
}

/// Get the address of the security cookie seed, given `address`
/// as the base, from the memory layout in `ctx` referenced by
/// `mem_layout_ref`, or `0` if no such memory layout exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_security_cookie_seed_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.guest_security_cookie_seed_offset)
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_guest_error_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_guest_error_offset())
    })
    .unwrap_or(0)
}

/// Get the address in guest memory of the guest error message size, or
/// `0` if no such memory layout exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_guest_error_message_size_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_guest_error_message_size_offset())
    })
    .unwrap_or(0)
}
/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_guest_error_message_pointer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_guest_error_message_pointer_offset())
    })
    .unwrap_or(0)
}
/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_function_definition_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_function_definition_offset())
    })
    .unwrap_or(0)
}

/// Get the memory layout in `ctx` referenced by `mem_layout_ref`,
/// then calculate the address of the host functions definition size
/// given `base_addr` was the start of memory.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_function_definition_size_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |ml| {
        Ok(ml.host_functions_offset)
    })
    .unwrap_or(0)
}
/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_function_definition_pointer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    // Pointer to functions data is after the size field which is a
    // ulong.
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_function_definition_pointer_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_host_exception_size_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.host_exception_offset)
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_host_exception_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.host_exception_buffer_offset)
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_out_b_pointer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_out_b_pointer_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_output_data_size_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_output_data_size_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_output_data_pointer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_output_data_pointer_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_output_data_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_output_data_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_input_data_size_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_input_data_size_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_input_data_pointer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_input_data_pointer_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_input_data_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(
        &*ctx,
        mem_layout_ref,
        base_addr,
        |l| Ok(l.input_data_offset),
    )
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_code_pointer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_code_pointer_offset())
    })
    .unwrap_or(0)
}
/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_dispatch_function_pointer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_dispatch_function_pointer_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_in_process_peb_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_in_process_peb_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_heap_size_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_heap_size_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_heap_pointer_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_heap_pointer_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_heap_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_heap_pointer_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_min_guest_stack_address_pointer(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_min_guest_stack_address_offset())
    })
    .unwrap_or(0)
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_top_of_stack_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
    base_addr: i64,
) -> i64 {
    impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
        Ok(l.get_top_of_stack_offset())
    })
    .unwrap_or(0)
}
/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_host_pml4_address(base_addr: i64) -> i64 {
    base_addr
}
/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_host_pdpt_address(base_addr: i64) -> i64 {
    Addr::from_i64(base_addr)
        .map(|addr| addr.add_usize(SandboxMemoryLayout::PDPT_OFFSET))
        .and_then(|addr| addr.as_i64())
        .unwrap_or(0)
}
/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_host_pd_address(base_addr: i64) -> i64 {
    Addr::from_i64(base_addr)
        .map(|addr| addr.add_usize(SandboxMemoryLayout::PD_OFFSET))
        .and_then(|addr| addr.as_i64())
        .unwrap_or(0)
}
/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_host_code_address(base_addr: i64) -> i64 {
    Addr::from_i64(base_addr)
        .map(|addr| addr.add_usize(SandboxMemoryLayout::CODE_OFFSET))
        .and_then(|addr| addr.as_i64())
        .unwrap_or(0)
}

/// Get the total size of the memory in the memory layout in
/// `ctx` referenced by `mem_layout_ref`.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_memory_size(
    ctx: *const Context,
    mem_layout_ref: Handle,
) -> usize {
    impls::get_memory_size(&*ctx, mem_layout_ref).unwrap_or(0)
}

/// Write the memory layout in `ctx` referenced by `mem_layout_ref`.
///
/// Returns an empty `Handle` if the write operation succeeded,
/// and an error `Handle` otherwise.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_layout_write_memory_layout(
    ctx: *mut Context,
    mem_layout_ref: Handle,
    guest_mem_ref: Handle,
    guest_address: usize,
    size: usize,
) -> Handle {
    match impls::write_memory_layout(&*ctx, mem_layout_ref, guest_mem_ref, guest_address, size) {
        Ok(_) => Handle::from(Hdl::Empty()),
        Err(e) => (*ctx).register_err(e),
    }
}
