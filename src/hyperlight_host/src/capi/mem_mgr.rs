use super::{byte_array::get_byte_array, context::Context, handle::Handle, hdl::Hdl};
use super::{guest_log_data::register_guest_log_data, shared_mem::get_shared_memory};
use crate::capi::function_call_result::get_function_call_result;
use crate::{
    capi::arrays::borrowed_slice::borrow_ptr_as_slice_mut,
    capi::{c_func::CFunc, int::register_i32},
    mem::{
        mgr::SandboxMemoryManager,
        ptr::{GuestPtr, HostPtr, RawPtr},
    },
};
use crate::{
    capi::int::register_u64,
    capi::strings::register_string,
    capi::{
        arrays::borrowed_slice::borrow_ptr_as_slice,
        bool::register_boolean,
        mem_layout::{get_mem_layout, register_mem_layout},
        shared_mem::register_shared_mem,
    },
    mem::{config::SandboxMemoryConfiguration, ptr_offset::Offset},
    validate_context, validate_context_or_panic,
};
use anyhow::{anyhow, bail, Result};

pub(super) fn get_mem_mgr(ctx: &Context, hdl: Handle) -> Result<&SandboxMemoryManager> {
    Context::get(hdl, &ctx.mem_mgrs, |h| matches!(h, Hdl::MemMgr(_))).map_err(|e| anyhow!(e))
}

fn get_mem_mgr_mut(ctx: &mut Context, hdl: Handle) -> Result<&mut SandboxMemoryManager> {
    Context::get_mut(hdl, &mut ctx.mem_mgrs, |h| matches!(h, Hdl::MemMgr(_)))
        .map_err(|e| anyhow!(e))
}

pub(crate) fn register_mem_mgr(ctx: &mut Context, mgr: SandboxMemoryManager) -> Handle {
    Context::register(mgr, &mut ctx.mem_mgrs, Hdl::MemMgr)
}

/// Macro to either get a `SandboxMemoryManager` from a `Handle` and
/// `Context`, or return a `Handle` referencing an error in the
/// same `Context`.
macro_rules! get_mgr {
    ($ctx:ident, $hdl: ident) => {
        match get_mem_mgr(&*$ctx, $hdl) {
            Ok(m) => m,
            Err(e) => return (*$ctx).register_err(e),
        }
    };
}

/// Validate a pointer to a flatbuffer points to a valid length-prefixed buffer and
/// then return a slice that points to the buffer.

fn validate_flatbuffer(fb_ptr: *const u8) -> Result<Vec<u8>> {
    if fb_ptr.is_null() {
        bail!("flat buffer pointer is NULL")
    }

    unsafe {
        borrow_ptr_as_slice(fb_ptr, 4, |outer_slice| {
            let len = usize::try_from(flatbuffers::read_scalar::<i32>(outer_slice) + 4)?;
            Ok(len)
        })
        .and_then(|len| borrow_ptr_as_slice(fb_ptr, len, |inner_slice| Ok(inner_slice.to_vec())))
    }
}

/// Create a new `SandboxMemoryManager` from the given `run_from_process`
/// memory and the `SandboxMemoryConfiguration` stored in `ctx` referenced by
/// `cfg_hdl`. Then, store it in `ctx`, and return a new `Handle` referencing
/// it.
///
/// This function should only be used for testing purposes.
///
/// TODO: delete this function after removing the entire C API
/// for SandboxMemoryManager.
///
/// # Safety
///
/// The called must pass a `ctx` to this function that was created
/// by `context_new`, not currently in use in any other function,
/// and not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_new(
    ctx: *mut Context,
    mem_cfg: SandboxMemoryConfiguration,
    shared_mem_hdl: Handle,
    layout_hdl: Handle,
    run_from_process_mem: bool,
    load_addr: u64,
    entrypoint_offset: u64,
) -> Handle {
    validate_context!(ctx);
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory(&*ctx, shared_mem_hdl) {
        Ok(s) => s,
        Err(e) => return (*ctx).register_err(e),
    };

    let mgr = SandboxMemoryManager::new(
        mem_cfg,
        layout,
        shared_mem.clone(),
        run_from_process_mem,
        RawPtr::from(load_addr),
        Offset::from(entrypoint_offset),
        #[cfg(target_os = "windows")]
        None,
    );
    Context::register(mgr, &mut (*ctx).mem_mgrs, Hdl::MemMgr)
}

/// Set the stack guard for the `SandboxMemoryManager` in `ctx` referenced
/// by `mgr_hdl`.
///
/// The location of the guard will be calculated using the `SandboxMemoryLayout`
/// in `ctx` referenced by `layout_hdl`, the contents of the stack guard
/// will be the byte array in `ctx` referenced by `cookie_hdl`, and the write
/// operations will be done with the `SharedMemory` in `ctx` referenced by
/// `shared_mem_hdl`.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_set_stack_guard(
    ctx: *mut Context,
    mgr_hdl: Handle,
    cookie_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let cookie = match get_byte_array(&*ctx, cookie_hdl) {
        Ok(c) => c,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.set_stack_guard(cookie) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Set up a new hypervisor partition in the given `Context` using the
/// `SharedMemory` referenced by `shared_mem_hdl`, the
/// `SandboxMemoryManager` referenced by `mgr_hdl`, and the given memory
/// size `mem_size`.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_set_up_hypervisor_partition(
    ctx: *mut Context,
    mgr_hdl: Handle,
    mem_size: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.set_up_hypervisor_partition(mem_size) {
        Ok(rsp) => Context::register(rsp, &mut (*ctx).uint64s, Hdl::UInt64),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Check the stack guard for the `SandboxMemoryManager` in `ctx` referenced
/// by `mgr_hdl`. Return a `Handle` referencing a boolean indicating
/// whether the stack guard matches the contents of the byte array
/// referenced by `cookie_hdl`. Otherwise, return a `Handle` referencing
/// an error.
///
/// The location of the guard will be calculated using the `SandboxMemoryLayout`
/// in `ctx` referenced by `layout_hdl`, the contents of the stack guard
/// will be the byte array in `ctx` referenced by `cookie_hdl`, and the write
/// operations will be done with the `SharedMemory` in `ctx` referenced by
/// `shared_mem_hdl`.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_check_stack_guard(
    ctx: *mut Context,
    mgr_hdl: Handle,
    cookie_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    let cookie = match get_byte_array(&*ctx, cookie_hdl) {
        Ok(c) => c,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.check_stack_guard(cookie) {
        Ok(res) => Context::register(res, &mut (*ctx).booleans, Hdl::Boolean),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get the address of the process environment block (PEB) and return a
/// `Handle` referencing it. On error, return a `Handle` referencing
/// that error. Use the `uint64` methods to fetch the returned value from
/// the returned `Handle`
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_peb_address(
    ctx: *mut Context,
    mem_mgr_hdl: Handle,
    mem_start_addr: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mem_mgr_hdl);
    let addr = match mgr.get_peb_address(mem_start_addr) {
        Ok(a) => a,
        Err(e) => return (*ctx).register_err(e),
    };
    Context::register(addr, &mut (*ctx).uint64s, Hdl::UInt64)
}

/// Fetch the `SandboxMemoryManager` referenced by `mgr_hdl`, then
/// snapshot the memory from the `SharedMemory` referenced by `shared_mem_hdl`
/// internally. Return an empty handle if all succeeded, and a `Handle`
/// referencing an error otherwise.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_snapshot_state(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.snapshot_state() {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Fetch the `SandboxMemoryManager` referenced by `mgr_hdl`, then
/// restore memory from the internally-stored snapshot. Return
/// an empty handle if the restore succeeded, and a `Handle` referencing
/// an error otherwise.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_restore_state(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.restore_state() {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get the return value of an executable that ran and return a `Handle`
/// referencing an int32 with the return value. Return a `Handle` referencing
/// an error otherwise.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_return_value(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    let ret_val = match mgr.get_return_value() {
        Ok(v) => v,
        Err(e) => return (*ctx).register_err(e),
    };
    register_i32(&mut *ctx, ret_val)
}

/// Sets `addr` to the correct offset in the memory referenced by
/// `shared_mem` to indicate the address of the outb pointer.
///
/// Return an empty `Handle` on success, and a `Handle` referencing
/// an error otherwise.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_set_outb_address(
    ctx: *mut Context,
    mgr_hdl: Handle,
    addr: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.set_outb_address(addr) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get the offset to use when calculating addresses.
///
/// Return a `Handle` referencing a uint64 on success, and a `Handle`
/// referencing an error otherwise.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_address_offset(
    ctx: *mut Context,
    mgr_hdl: Handle,
    source_addr: u64,
) -> Handle {
    CFunc::new("mem_mgr_get_address_offset", ctx)
        .and_then(|c, _| {
            let mgr = get_mem_mgr(c, mgr_hdl)?;
            Ok(mgr.get_address_offset(source_addr))
        })
        .map_static_mut(register_u64)
        .ok_or_err_hdl()
}

/// Translate `addr` -- a pointer to memory in the guest address space --
/// to the equivalent pointer in the host's.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_host_address_from_pointer(
    ctx: *mut Context,
    mgr_hdl: Handle,
    addr: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    let guest_ptr = match GuestPtr::try_from(RawPtr::from(addr)) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.get_host_address_from_ptr(guest_ptr) {
        Ok(addr_ptr) => match addr_ptr.absolute() {
            Ok(addr) => register_u64(&mut *ctx, addr),
            Err(e) => (*ctx).register_err(e),
        },
        Err(e) => (*ctx).register_err(e),
    }
}

/// Translate `addr` -- a pointer to memory in the host's address space --
/// to the equivalent pointer in the guest's.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_guest_address_from_pointer(
    ctx: *mut Context,
    mgr_hdl: Handle,
    addr: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);

    let host_ptr = match HostPtr::try_from((RawPtr::from(addr), &mgr.shared_mem)) {
        Ok(p) => p,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.get_guest_address_from_ptr(host_ptr) {
        Ok(addr_ptr) => match addr_ptr.absolute() {
            Ok(addr) => register_u64(&mut *ctx, addr),
            Err(e) => (*ctx).register_err(e),
        },
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get the address of the dispatch function located in the guest memory
/// referenced by `shared_mem_hdl`.
///
/// On success, return a new `Handle` referencing a uint64 in memory. On
/// failure, return a new `Handle` referencing an error.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_pointer_to_dispatch_function(
    ctx: *mut Context,
    mgr_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    match mgr.get_pointer_to_dispatch_function() {
        Ok(ptr) => register_u64(&mut *ctx, ptr),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Use `SandboxMemoryManager` in `ctx` referenced
/// by `mgr_hdl` to get a string value written to output by the Hyperlight Guest
/// Return a `Handle` referencing the string contents. Otherwise, return a `Handle` referencing
/// an error.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_read_string_output(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);

    match mgr.get_string_output() {
        Ok(output) => register_string(&mut *ctx, output),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Use `SandboxMemoryManager` in `ctx` referenced
/// by `mgr_hdl` to get a boolean if an exception was written by the Hyperlight Host
/// Returns a `Handle` containing a bool that describes if exception data exists or a `Handle` referencing an error.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_has_host_exception(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    match mgr.has_host_exception() {
        Ok(output) => Context::register(output, &mut (*ctx).booleans, Hdl::Boolean),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Use `SandboxMemoryManager` in `ctx` referenced
/// by `mgr_hdl` to get the length of any exception data that was written by the Hyperlight Host
/// Returns a `Handle` containing a i32 representing the length of the exception data or a `Handle` referencing an error.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_host_exception_length(
    ctx: *mut Context,
    mgr_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    match mgr.get_host_exception_length() {
        Ok(output) => Context::register(output, &mut (*ctx).int32s, Hdl::Int32),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Use `SandboxMemoryManager` in `ctx` referenced
/// by `mgr_hdl` to get the exception data that was written by the Hyperlight Host
/// Returns an Empty `Handle` or a `Handle` referencing an error.
/// Writes the exception data to the buffer at `exception_data_ptr` for length `exception_data_len`, `exception_data_ptr`
/// should be a pointer to contiguous memory of length ``exception_data_len`.
/// The caller is responsible for allocating and free the memory buffer.
/// The length of the buffer must match the length of the exception data available, the length can be
/// determind by calling `mem_mgr_get_host_exception_length`
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
/// `exception_data_ptr` must be a valid pointer to a buffer of size `exception_data_len`, this buffer is owned and managed by the client.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_host_exception_data(
    ctx: *mut Context,
    mgr_hdl: Handle,
    exception_data_ptr: *mut u8,
    exception_data_len: i32,
) -> Handle {
    CFunc::new("mem_mgr_get_host_exception_data", ctx)
        .and_then(|c, _| {
            let mgr = get_mem_mgr(c, mgr_hdl)?;
            if exception_data_ptr.is_null() {
                bail!("Exception data ptr is null");
            }
            if exception_data_len == 0 {
                bail!("Exception data length is zero");
            }
            let exception_data_len_usize = usize::try_from(exception_data_len).map_err(|_| {
                anyhow!(
                    "converting exception_data_len ({:?}) to usize",
                    exception_data_len
                )
            })?;
            borrow_ptr_as_slice_mut(exception_data_ptr, exception_data_len_usize, |slice| {
                mgr.get_host_exception_data(slice)
            })?;
            Ok(Handle::new_empty())
        })
        .ok_or_err_hdl()
}

/// Use `SandboxMemoryManager` in `ctx` referenced by `mgr_hdl` to write a guest error message and
/// host exception data when an exception occurs processing a guest request in the host.
///
/// When the guest calls a function in the host an error may occur, these errors cannot be transparently handled,so the host signals the error by writing
/// an error code (`OUTB_ERROR` ) and error message to the guest error section of shared memory, it also serialises any exception
/// data into the host exception section. When the call returns from the host , the guests checks to see if an error occurs
/// and if so returns control to the host which can then check for an `OUTB_ERROR` and read the exception data and
/// process it
///
/// Returns an Empty `Handle` or a `Handle` referencing an error.
/// Writes the an `OUTB_ERROR` code along with guest error message from the `guest_error_msg_hdl` to memory, writes the host exception data
/// from the `host_exception_hdl` to memory.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_write_outb_exception(
    ctx: *mut Context,
    mgr_hdl: Handle,
    guest_error_msg_hdl: Handle,
    host_exception_data_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_error_msg = match get_byte_array(&*ctx, guest_error_msg_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let host_exception_data = match get_byte_array(&*ctx, host_exception_data_hdl) {
        Ok(h) => h,
        Err(e) => return (*ctx).register_err(e),
    };

    match mgr.write_outb_exception(guest_error_msg, host_exception_data) {
        Ok(_) => Handle::from(Hdl::Empty()),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Use `SandboxMemoryManager` in `ctx` referenced by `mgr_hdl` to get guest error details from shared memory.
///
///
/// Returns an Empty `Handle` to a `GuestError` or a `Handle` referencing an error.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_guest_error(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    match mgr.get_guest_error() {
        Ok(output) => Context::register(output, &mut (*ctx).guest_errors, Hdl::GuestError),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get the offset to the entrypoint in the `SandboxMemoryManager` in
/// `ctx` referenced by `mgr_hdl`.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.

#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_entrypoint_offset(
    ctx: *mut Context,
    mgr_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    let val = mgr.entrypoint_offset;
    register_u64(&mut *ctx, val.into())
}

/// Get a new `Handle` referencing the `SharedMemory` in `ctx` referenced
/// by the given `mgr_hdl`
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_shared_memory(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    let val = mgr.shared_mem.clone();
    register_shared_mem(&mut *ctx, val)
}

/// Get a new `Handle` referencing the uint64 load address for the
/// `SandboxMemoryManager` in `ctx` referenced by the given `mgr_hdl`
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.

#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_load_addr(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    let val = &mgr.load_addr;
    register_u64(&mut *ctx, val.into())
}

/// Get a new `Handle` referencing the `SandboxMemoryLayout` for the
/// `SandboxMemoryManager` in `ctx` referenced by the given `mgr_hdl`
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_sandbox_memory_layout(
    ctx: *mut Context,
    mgr_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    register_mem_layout(&mut *ctx, mgr.layout)
}

/// Get a new `Handle` referencing the bool indicating whether to
/// run the binary from process memory or not for the
/// `SandboxMemoryManager` in `ctx` referenced by the given `mgr_hdl`
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_run_from_process_memory(
    ctx: *mut Context,
    mgr_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    register_boolean(&mut *ctx, mgr.run_from_process_memory)
}

/// Get the `SandboxMemoryConfiguration` for the `SandboxMemoryManager`
/// in `ctx` referenced by the given `mgr_hdl`
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_config(
    ctx: *mut Context,
    mgr_hdl: Handle,
) -> SandboxMemoryConfiguration {
    validate_context_or_panic!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(_) => panic!("mem_mgr_get_config invalid handle"),
    };
    mgr.mem_cfg
}

/// Get a new `Handle` referencing the uint64 memory size for the
/// `SandboxMemoryManager` in `ctx` referenced by the given `mgr_hdl`
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_mem_size(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    validate_context!(ctx);
    let mgr = get_mgr!(ctx, mgr_hdl);
    let val_usize = mgr.shared_mem.mem_size();
    let val = match u64::try_from(val_usize) {
        Ok(s) => s,
        Err(_) => {
            return (*ctx).register_err(anyhow!(
                "mem_mgr_get_mem_size couldn't convert usize mem size ({}) to u64",
                val_usize,
            ))
        }
    };
    register_u64(&mut *ctx, val)
}

/// Writes the data pointed to by `fb_guest_function_call_ptr` as a `FunctionCall` flatbuffer to shared memory.
/// The buffer should contain a valid size prefixed FunctionCall flatbuffer representing a Guest Function Call.
///
/// Return an empty `Handle` on success, and a `Handle` referencing
/// an error otherwise.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
///
/// `mem_mgr_hdl` must be a valid `Handle` returned by `mem_mgr_new` and associated with the `ctx`
///
/// `fb_guest_function_call_ptr` must be a pointer to a valid size prefixed flatbuffer containing a `FunctionCall` flatbuffer , the FunctionCall buffer should represent a Guest FunctionCall, it is owned by the caller.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_write_guest_function_call(
    ctx: *mut Context,
    mem_mgr_hdl: Handle,
    fb_guest_function_call_ptr: *const u8,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mem_mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    match validate_flatbuffer(fb_guest_function_call_ptr) {
        Ok(vec) => match mgr.write_guest_function_call(&vec) {
            Ok(_) => Handle::new_empty(),
            Err(e) => (*ctx).register_err(e),
        },
        Err(e) => (*ctx).register_err(e),
    }
}

/// Writes the data pointed to by `fb_host_function_details_ptr` as a `HostFunctionDetails` flatbuffer to shared memory.
/// The buffer should contain a valid size prefixed HostFunctionDetails flatbuffer
///
/// Return an empty `Handle` on success, and a `Handle` referencing
/// an error otherwise.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
///
/// `mem_mgr_hdl` must be a valid `Handle` returned by `mem_mgr_new` and associated with the `ctx`
///
/// `fb_host_function_details_ptr` must be a pointer to a valid size prefixed flatbuffer containing a `HostFunctionDetails` flatbuffer , it is owned by the caller.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_write_host_function_details(
    ctx: *mut Context,
    mem_mgr_hdl: Handle,
    fb_host_function_details_ptr: *const u8,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mem_mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };

    match validate_flatbuffer(fb_host_function_details_ptr) {
        Ok(vec) => match mgr.write_host_function_details(&vec) {
            Ok(_) => Handle::new_empty(),
            Err(e) => (*ctx).register_err(e),
        },
        Err(e) => (*ctx).register_err(e),
    }
}

/// Writes the data pointed to by `fb_host_function_call_ptr` as a `FunctionCall` flatbuffer to shared memory.
/// The buffer should contain a valid size prefixed FunctionCall flatbuffer representing a Host Function Call.
///
/// Return an empty `Handle` on success, and a `Handle` referencing
/// an error otherwise.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
///
/// `mem_mgr_hdl` must be a valid `Handle` returned by `mem_mgr_new` and associated with the `ctx`
///
/// `fb_host_function_call_ptr` must be a pointer to a valid size prefixed flatbuffer containing a `FunctionCall` flatbuffer , the FunctionCall buffer should represent a Host FunctionCall, it is owned by the caller.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_write_host_function_call(
    ctx: *mut Context,
    mem_mgr_hdl: Handle,
    fb_host_function_call_ptr: *const u8,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mem_mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    match validate_flatbuffer(fb_host_function_call_ptr) {
        Ok(vec) => match mgr.write_host_function_call(&vec) {
            Ok(_) => Handle::new_empty(),
            Err(e) => (*ctx).register_err(e),
        },
        Err(e) => (*ctx).register_err(e),
    }
}

/// Write the response from a host function call to memory.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_write_response_from_host_method_call(
    ctx: *mut Context,
    mem_mgr_hdl: Handle,
    function_call_result_hdl: Handle,
) -> Handle {
    let mem_mgr = match get_mem_mgr_mut(&mut *ctx, mem_mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let func_call_res = match get_function_call_result(&*ctx, function_call_result_hdl) {
        Ok(f) => f,
        Err(e) => return (*ctx).register_err(e),
    };

    match mem_mgr.write_response_from_host_method_call(func_call_res) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Use `SandboxMemoryManager` in `ctx` referenced by `mgr_hdl` to get host function call details from shared memory.
///
///
/// Returns a `Handle` to a `FunctionCall` or a `Handle` referencing an error.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
///
/// `mem_mgr_hdl` must be a valid `Handle` returned by `mem_mgr_new` and associated with the `ctx`
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_host_function_call(
    ctx: *mut Context,
    mem_mgr_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mem_mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.get_host_function_call() {
        Ok(output) => Context::register(
            output,
            &mut (*ctx).host_function_calls,
            Hdl::HostFunctionCall,
        ),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Use `SandboxMemoryManager` in `ctx` referenced by `mgr_hdl` to get function call result details from shared memory.
///
/// Returns a `Handle` to a `FunctionCallResult` or a `Handle` referencing an error.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
///
/// `mem_mgr_hdl` must be a valid `Handle` returned by `mem_mgr_new` and associated with the `ctx`
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_function_call_result(
    ctx: *mut Context,
    mgr_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.get_function_call_result() {
        Ok(output) => Context::register(
            output,
            &mut (*ctx).function_call_results,
            Hdl::FunctionCallResult,
        ),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Read the `GuestLogData` from the `SandboxMemoryManager` in `ctx` referenced
/// by `mgr_hdl`, then return a new `Handle` referencing the new `GuestLogData`.
/// Return a new `Handle` referencing an error if something went wrong
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_read_guest_log_data(ctx: *mut Context, mgr_hdl: Handle) -> Handle {
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_log_data = match mgr.read_guest_log_data() {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    register_guest_log_data(&mut *ctx, guest_log_data)
}
