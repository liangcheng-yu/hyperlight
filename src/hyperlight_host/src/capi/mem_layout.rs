use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::Addr;
use crate::mem::{config::SandboxMemoryConfiguration, layout::SandboxMemoryLayout};
use crate::{validate_context, validate_context_or_panic};
use anyhow::anyhow;

mod impls {
    use crate::capi::guest_mem::get_guest_memory_mut;
    use crate::capi::handle::Handle;
    use crate::capi::hdl::Hdl;
    use crate::capi::Addr;
    use crate::mem::layout::SandboxMemoryLayout;
    use crate::{capi::context::Context, mem::config::SandboxMemoryConfiguration};
    use anyhow::{anyhow, Result};

    pub fn new(
        mem_cfg: SandboxMemoryConfiguration,
        code_size: usize,
        stack_size: usize,
        heap_size: usize,
    ) -> Result<SandboxMemoryLayout> {
        Ok(SandboxMemoryLayout::new(
            mem_cfg, code_size, stack_size, heap_size,
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
        let layout = get_mem_layout(ctx, layout_ref)?;
        let base = Addr::from_i64(base)?;
        let offset = fetcher_fn(&layout)?;
        base.add_usize(offset).as_i64()
    }

    pub fn get_memory_size(ctx: &Context, mem_layout_ref: Handle) -> Result<usize> {
        let layout = get_mem_layout(ctx, mem_layout_ref)?;
        layout.get_memory_size()
    }

    pub fn write_memory_layout(
        ctx: &mut Context,
        mem_layout_ref: Handle,
        guest_mem_ref: Handle,
        guest_offset: usize,
        size: usize,
    ) -> Result<()> {
        let layout = get_mem_layout(ctx, mem_layout_ref)?;
        let guest_mem = get_guest_memory_mut(ctx, guest_mem_ref)?;
        layout.write(&mut (*guest_mem), guest_offset, size)
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
}

pub use impls::get_mem_layout;

/// Create a new memory layout within `ctx` with the given parameters and
/// return a reference to it.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn mem_layout_new(
    ctx: *mut Context,
    mem_cfg: SandboxMemoryConfiguration,
    code_size: usize,
    stack_size: usize,
    heap_size: usize,
) -> Handle {
    validate_context!(ctx);

    match std::panic::catch_unwind(|| {
        let _ = mem_cfg.guest_error_message_size;
        let _ = mem_cfg.host_function_definition_size;
        let _ = mem_cfg.host_exception_size;
        let _ = mem_cfg.input_data_size;
        let _ = mem_cfg.output_data_size;
    }) {
        Ok(_) => (),
        Err(_) => {
            return (*ctx).register_err(anyhow!("SandboxMemoryConfiguration struct is invalid"))
        }
    };

    match impls::new(mem_cfg, code_size, stack_size, heap_size) {
        Ok(layout) => Context::register(layout, &mut (*ctx).mem_layouts, Hdl::MemLayout),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Convenience macro to create mem_layout_get_$something_address functions using field from `SandboxMemoryLayout`.
macro_rules! mem_layout_get_address_using_field {
    ($something:ident) => {
        paste::item! {
            /// Get the address for a requested resource from the memory layout in `ctx`
            /// referenced by `mem_layout_ref`, or `0` if no such memory layout
            /// exists.
            ///
            /// # Safety
            ///
            /// You must call this function with
            ///
            /// - A `Context*` that has been:
            ///     - Created with `context_new`
            ///     - Not yet freed with `context_free`
            ///     - Not modified, except by calling functions in the Hyperlight C API
            /// - A valid handle to a memory layout
            /// - A valid base memory address
            #[no_mangle]
            pub unsafe extern "C" fn [< mem_layout_get_ $something _address >](
                ctx: *const Context,
                mem_layout_ref: Handle,
                base_addr: i64,
            ) -> i64 {
                validate_context_or_panic!(ctx);

                impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
                    Ok(l.[< $something _offset >])
                })
                .unwrap_or(0)
            }
        }
    };
}

mem_layout_get_address_using_field!(code_and_outb_pointer);
mem_layout_get_address_using_field!(guest_error);
mem_layout_get_address_using_field!(guest_error_message_buffer);
mem_layout_get_address_using_field!(host_function_definitions);
mem_layout_get_address_using_field!(input_data_buffer);
mem_layout_get_address_using_field!(output_data_buffer);

/// A convenience macro to create mem_layout_get_$something_address functions
/// using method from `SandboxMemoryLayout`.
macro_rules! mem_layout_get_address_using_method {
    ($something:ident) => {
        paste::item! {
            /// Get the offset for a requested resource from the memory layout in `ctx`
            /// referenced by `mem_layout_ref`, or `0` if no such memory layout
            /// exists.
            ///
            /// # Safety
            ///
            /// You must call this function with
            ///
            /// - A `Context*` that has been:
            ///     - Created with `context_new`
            ///     - Not yet freed with `context_free`
            ///     - Not modified, except by calling functions in the Hyperlight C API
            /// - A valid handle to a memory layout
            /// - A valid base memory address
            #[no_mangle]
            pub unsafe extern "C" fn [< mem_layout_get_ $something _address >](
                ctx: *const Context,
                mem_layout_ref: Handle,
                base_addr: i64,
            ) -> i64 {
                validate_context_or_panic!(ctx);

                impls::calculate_address(&*ctx, mem_layout_ref, base_addr, |l| {
                    Ok(l.[< get_ $something _offset >] ())
                })
                .unwrap_or(0)
            }
        }
    };
}

mem_layout_get_address_using_method!(code_pointer);
mem_layout_get_address_using_method!(dispatch_function_pointer);
mem_layout_get_address_using_method!(guest_error_message_size);
mem_layout_get_address_using_method!(guest_error_message_pointer);
mem_layout_get_address_using_method!(heap_size);
mem_layout_get_address_using_method!(host_exception);
mem_layout_get_address_using_method!(host_exception_size);
mem_layout_get_address_using_method!(host_function_definitions_pointer);
mem_layout_get_address_using_method!(host_function_definitions_size);
mem_layout_get_address_using_method!(input_data_size);
mem_layout_get_address_using_method!(in_process_peb);
mem_layout_get_address_using_method!(output_data_size);
mem_layout_get_address_using_method!(output_data);
mem_layout_get_address_using_method!(top_of_stack);

/// Get the stack size from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` an error occurs.
///
/// # Safety
///
/// You must call this function with
///
/// - A `Context*` that has been:
///     - Created with `context_new`
///     - Not yet freed with `context_free`
///     - Not modified, except by calling functions in the Hyperlight C API
/// - A valid handle to a memory layout
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_stack_size(
    ctx: *const Context,
    mem_layout_ref: Handle,
) -> usize {
    validate_context_or_panic!(ctx);

    match impls::get_mem_layout(&*ctx, mem_layout_ref) {
        Ok(l) => l.stack_size,
        Err(_) => 0,
    }
}

/// Get the peb address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with
///
/// - A `Context*` that has been:
///     - Created with `context_new`
///     - Not yet freed with `context_free`
///     - Not modified, except by calling functions in the Hyperlight C API
/// - A valid handle to a memory layout
/// Note: This gets the `peb_address`, not `peb_offset`.
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_peb_address(
    ctx: *const Context,
    mem_layout_ref: Handle,
) -> i64 {
    validate_context_or_panic!(ctx);

    // NOTE: using calculate_address here for the safe conversions
    // from usize -> i64, but not for calculating offsets, as we
    // do in most of the other functions herein.
    impls::calculate_address(&*ctx, mem_layout_ref, 0, |l| Ok(l.peb_address)).unwrap_or(0)
}

/// Get the VMs base address from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_base_address() -> usize {
    SandboxMemoryLayout::BASE_ADDRESS
}

/// Get the pml4 offset from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_pml4_offset() -> usize {
    SandboxMemoryLayout::PML4_OFFSET
}

/// Get the pd offset from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_pd_offset() -> usize {
    SandboxMemoryLayout::PD_OFFSET
}

/// Get the pdpt offset from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_pdpt_offset() -> usize {
    SandboxMemoryLayout::PDPT_OFFSET
}

/// Get the size of the page tables from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_page_table_size() -> usize {
    SandboxMemoryLayout::PAGE_TABLE_SIZE
}

/// Get the host code address from the memory layout in `ctx`
/// referenced by `mem_layout_ref`, or `0` if no such memory layout
/// exists.
///
/// # Safety
///
/// You must call this function with
///
/// - A valid base memory address
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
/// You must call this function with
///
/// - A `Context*` that has been:
///     - Created with `context_new`
///     - Not yet freed with `context_free`
///     - Not modified, except by calling functions in the Hyperlight C API
/// - A valid handle to a memory layout
#[no_mangle]
pub unsafe extern "C" fn mem_layout_get_memory_size(
    ctx: *const Context,
    mem_layout_ref: Handle,
) -> usize {
    validate_context_or_panic!(ctx);

    // At present this is masking the error from get_memory_size so the client doesnt get to know what the failure was
    // The only time this call fails is if the memory requested exceeds the limit (which is hardcoded at the moment)
    // TODO: convert this to a handle so that a detailed error can be returned
    impls::get_memory_size(&*ctx, mem_layout_ref).unwrap_or(0)
}

/// Write the memory layout in `ctx` referenced by `mem_layout_ref`.
///
/// Returns an empty `Handle` if the write operation succeeded,
/// and an error `Handle` otherwise.
///
/// # Safety
///
/// You must call this function with
///
/// - A `Context*` that has been:
///     - Created with `context_new`
///     - Not yet freed with `context_free`
///     - Not modified, except by calling functions in the Hyperlight C API
/// - A valid handle to a memory layout
/// - A valid handle to guest memory
/// - The guest base address offset to apply to addresses in the memory layout
/// - The size of the guest memory
#[no_mangle]
pub unsafe extern "C" fn mem_layout_write_memory_layout(
    ctx: *mut Context,
    mem_layout_ref: Handle,
    guest_mem_ref: Handle,
    guest_address: usize,
    size: usize,
) -> Handle {
    validate_context!(ctx);

    match impls::write_memory_layout(
        &mut *ctx,
        mem_layout_ref,
        guest_mem_ref,
        guest_address,
        size,
    ) {
        Ok(_) => Handle::from(Hdl::Empty()),
        Err(e) => (*ctx).register_err(e),
    }
}
