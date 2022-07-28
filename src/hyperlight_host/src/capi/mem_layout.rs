use crate::mem::layout::SandboxMemoryLayout;

use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;

mod impls {
    use crate::capi::context::Context;
    use crate::capi::handle::Handle;
    use crate::capi::mem_config::get_mem_config;
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

#[repr(C)]
pub struct SandboxMemoryLayoutView {
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

impl From<SandboxMemoryLayout> for SandboxMemoryLayoutView {
    fn from(layout: SandboxMemoryLayout) -> Self {
        SandboxMemoryLayoutView {
            peb_offset: layout.peb_offset,
            stack_size: layout.stack_size,
            heap_size: layout.heap_size,
            host_functions_offset: layout.host_functions_offset,
            host_exception_offset: layout.host_exception_offset,
            guest_error_message_offset: layout.guest_error_message_offset,
            code_and_outb_pointer_offset: layout.code_and_outb_pointer_offset,
            input_data_offset: layout.input_data_offset,
            output_data_offset: layout.output_data_offset,
            heap_data_offset: layout.heap_data_offset,
            stack_data_offset: layout.stack_data_offset,
            code_size: layout.code_size,
            host_functions_buffer_offset: layout.host_functions_buffer_offset,
            host_exception_buffer_offset: layout.host_exception_buffer_offset,
            guest_error_message_buffer_offset: layout.guest_error_message_buffer_offset,
            input_data_buffer_offset: layout.input_data_buffer_offset,
            output_data_buffer_offset: layout.output_data_buffer_offset,
            guest_heap_buffer_offset: layout.guest_heap_buffer_offset,
            guest_stack_buffer_offset: layout.guest_stack_buffer_offset,
            peb_address: layout.peb_address,
        }
    }
}

/// Get the memory layout in `ctx` referenced by `mem_layout_ref`
/// and return a pointer to it. This pointer is valid as long as
/// `mem_layout_ref` is valid, and it must not be modified or deleted.
///
/// Returns `NULL` if `mem_layout_ref` is invalid.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
///
/// Also, if this function returns a pointer that is not `NULL`,
/// it has created new memory that you are responsible for freeing
/// with `free()` when you're done.
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get(
    ctx: *const Context,
    mem_layout_ref: Handle,
) -> *const SandboxMemoryLayoutView {
    let ctx_ref = &*ctx;
    match Context::get(mem_layout_ref, &ctx_ref.mem_layouts, |h| {
        matches!(h, Hdl::MemLayout(_))
    }) {
        Ok(layout) => {
            let layout_view = Box::new(SandboxMemoryLayoutView::from(*layout));
            Box::into_raw(layout_view)
        }
        Err(_) => std::ptr::null(),
    }
}
