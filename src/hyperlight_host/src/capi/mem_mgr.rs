use super::mem_layout::get_mem_layout;
use super::shared_mem::{get_shared_memory, get_shared_memory_mut};
use super::{byte_array::get_byte_array, context::Context, handle::Handle, hdl::Hdl};
use crate::{capi::int::register_u64, capi::strings::register_string, validate_context};
use crate::{
    capi::{arrays::borrowed_slice::borrow_ptr_as_slice_mut, int::register_i32},
    mem::{
        config::SandboxMemoryConfiguration,
        mgr::SandboxMemoryManager,
        ptr::{GuestPtr, HostPtr, RawPtr},
    },
};
use anyhow::{anyhow, Result};

fn get_mem_mgr(ctx: &Context, hdl: Handle) -> Result<&SandboxMemoryManager> {
    Context::get(hdl, &ctx.mem_mgrs, |h| matches!(h, Hdl::MemMgr(_))).map_err(|e| anyhow!(e))
}

fn get_mem_mgr_mut(ctx: &mut Context, hdl: Handle) -> Result<&mut SandboxMemoryManager> {
    Context::get_mut(hdl, &mut ctx.mem_mgrs, |h| matches!(h, Hdl::MemMgr(_)))
        .map_err(|e| anyhow!(e))
}

/// Create a new `SandboxMemoryManager` from the given `run_from_process`
/// memory and the `SandboxMemoryConfiguration` stored in `ctx` referenced by
/// `cfg_hdl`. Then, store it in `ctx`, and return a new `Handle` referencing
/// it.
///
/// # Safety
///
/// The called must pass a `ctx` to this function that was created
/// by `context_new`, not currently in use in any other function,
/// and not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_new(
    ctx: *mut Context,
    cfg: SandboxMemoryConfiguration,
    run_from_process_mem: bool,
) -> Handle {
    validate_context!(ctx);
    let mgr = SandboxMemoryManager::new(cfg, run_from_process_mem);
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
    layout_hdl: Handle,
    shared_mem_hdl: Handle,
    cookie_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory_mut(&mut *ctx, shared_mem_hdl) {
        Ok(gm) => gm,
        Err(e) => return (*ctx).register_err(e),
    };
    let cookie = match get_byte_array(&*ctx, cookie_hdl) {
        Ok(c) => c,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.set_stack_guard(&layout, shared_mem, cookie) {
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
    shared_mem_hdl: Handle,
    mem_size: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory_mut(&mut *ctx, shared_mem_hdl) {
        Ok(gm) => gm,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.set_up_hypervisor_partition(shared_mem, mem_size) {
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
    layout_hdl: Handle,
    shared_mem_hdl: Handle,
    cookie_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory(&*ctx, shared_mem_hdl) {
        Ok(gm) => gm,
        Err(e) => return (*ctx).register_err(e),
    };
    let cookie = match get_byte_array(&*ctx, cookie_hdl) {
        Ok(c) => c,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.check_stack_guard(&layout, shared_mem, cookie) {
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
    mem_layout_hdl: Handle,
    mem_start_addr: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mem_mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, mem_layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    let addr = match mgr.get_peb_address(&layout, mem_start_addr) {
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
pub unsafe extern "C" fn mem_mgr_snapshot_state(
    ctx: *mut Context,
    mgr_hdl: Handle,
    shared_mem_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory(&*ctx, shared_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };

    match mgr.snapshot_state(shared_mem) {
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
pub unsafe extern "C" fn mem_mgr_get_return_value(
    ctx: *mut Context,
    mgr_hdl: Handle,
    shared_mem_hdl: Handle,
    layout_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory(&*ctx, shared_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    let ret_val = match mgr.get_return_value(shared_mem, &layout) {
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
    shared_mem_hdl: Handle,
    layout_hdl: Handle,
    addr: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory_mut(&mut *ctx, shared_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.set_outb_address(shared_mem, &layout, addr) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get the name of the method called by the host.
///
/// Return a `Handle` referencing a `string` with the method name,
/// or a `Handle` referencing an error if something went wrong.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn mem_mgr_get_host_call_method_name(
    ctx: *mut Context,
    mgr_hdl: Handle,
    guest_mem_hdl: Handle,
    layout_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_shared_memory(&*ctx, guest_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };

    match mgr.get_host_call_method_name(guest_mem, &layout) {
        Ok(method_name) => register_string(&mut *ctx, method_name),
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
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let val = mgr.get_address_offset(source_addr);
    register_u64(&mut *ctx, val)
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
    shared_mem_hdl: Handle,
    addr: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory(&*ctx, shared_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_ptr = match GuestPtr::try_from((RawPtr::from(addr), mgr.run_from_process_memory)) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.get_host_address_from_ptr(guest_ptr, shared_mem) {
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
    shared_mem_hdl: Handle,
    addr: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory(&*ctx, shared_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };

    let host_ptr =
        match HostPtr::try_from((RawPtr::from(addr), shared_mem, mgr.run_from_process_memory)) {
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
    shared_mem_hdl: Handle,
    layout_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory(&*ctx, shared_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.get_pointer_to_dispatch_function(shared_mem, &layout) {
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
pub unsafe extern "C" fn mem_mgr_read_string_output(
    ctx: *mut Context,
    mgr_hdl: Handle,
    layout_hdl: Handle,
    shared_mem_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let shared_mem = match get_shared_memory_mut(&mut *ctx, shared_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };

    match mgr.get_string_output(&layout, shared_mem) {
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
pub unsafe extern "C" fn mem_mgr_has_host_exception(
    ctx: *mut Context,
    mgr_hdl: Handle,
    layout_hdl: Handle,
    guest_mem_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_shared_memory_mut(&mut *ctx, guest_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.has_host_exception(&layout, guest_mem) {
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
    layout_hdl: Handle,
    guest_mem_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_shared_memory_mut(&mut *ctx, guest_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.get_host_exception_length(&layout, guest_mem) {
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
    layout_hdl: Handle,
    guest_mem_hdl: Handle,
    exception_data_ptr: *mut u8,
    exception_data_len: i32,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_shared_memory_mut(&mut *ctx, guest_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    if exception_data_ptr.is_null() {
        return (*ctx).register_err(anyhow!("Exception data ptr is null"));
    }
    if exception_data_len == 0 {
        return (*ctx).register_err(anyhow!("Exception data length is zero"));
    }
    let exception_data_len_usize = match usize::try_from(exception_data_len) {
        Ok(l) => l,
        Err(_) => {
            return (*ctx).register_err(anyhow!(
                "converting exception_data_len ({:?}) to usize",
                exception_data_len
            ))
        }
    };
    match borrow_ptr_as_slice_mut(exception_data_ptr, exception_data_len_usize, |slice| {
        mgr.get_host_exception_data(&layout, guest_mem, slice)
    }) {
        Ok(_) => Handle::from(Hdl::Empty()),
        Err(e) => (*ctx).register_err(e),
    }
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
    layout_hdl: Handle,
    guest_mem_hdl: Handle,
    guest_error_msg_hdl: Handle,
    host_exception_data_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_shared_memory_mut(&mut *ctx, guest_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
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

    match mgr.write_outb_exception(&layout, guest_mem, guest_error_msg, host_exception_data) {
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
pub unsafe extern "C" fn mem_mgr_get_guest_error(
    ctx: *mut Context,
    mgr_hdl: Handle,
    layout_hdl: Handle,
    guest_mem_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_shared_memory_mut(&mut *ctx, guest_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.get_guest_error(&layout, guest_mem) {
        Ok(output) => Context::register(output, &mut (*ctx).guest_errors, Hdl::GuestError),
        Err(e) => (*ctx).register_err(e),
    }
}
