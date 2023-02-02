use super::guest_mem::{get_guest_memory, get_guest_memory_mut};
use super::mem_layout::get_mem_layout;
use super::{byte_array::get_byte_array, context::Context, handle::Handle, hdl::Hdl};
use crate::{
    capi::int::register_i32,
    mem::{config::SandboxMemoryConfiguration, mgr::SandboxMemoryManager},
};
use crate::{capi::int::register_u64, validate_context};
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
/// operations will be done with the `GuestMemory` in `ctx` referenced by
/// `guest_mem_hdl`.
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
    guest_mem_hdl: Handle,
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
    let guest_mem = match get_guest_memory_mut(&mut *ctx, guest_mem_hdl) {
        Ok(gm) => gm,
        Err(e) => return (*ctx).register_err(e),
    };
    let cookie = match get_byte_array(&*ctx, cookie_hdl) {
        Ok(c) => c,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.set_stack_guard(&layout, guest_mem, cookie) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Set up a new hypervisor partition in the given `Context` using the
/// `GuestMemory` referenced by `guest_mem_hdl`, the
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
    guest_mem_hdl: Handle,
    mem_size: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_guest_memory_mut(&mut *ctx, guest_mem_hdl) {
        Ok(gm) => gm,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.set_up_hypervisor_partition(guest_mem, mem_size) {
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
/// operations will be done with the `GuestMemory` in `ctx` referenced by
/// `guest_mem_hdl`.
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
    guest_mem_hdl: Handle,
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
    let guest_mem = match get_guest_memory(&*ctx, guest_mem_hdl) {
        Ok(gm) => gm,
        Err(e) => return (*ctx).register_err(e),
    };
    let cookie = match get_byte_array(&*ctx, cookie_hdl) {
        Ok(c) => c,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.check_stack_guard(&layout, guest_mem, cookie) {
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
    let addr = mgr.get_peb_address(&layout, mem_start_addr);
    Context::register(addr, &mut (*ctx).uint64s, Hdl::UInt64)
}

/// Fetch the `SandboxMemoryManager` referenced by `mgr_hdl`, then
/// snapshot the memory from the `GuestMemory` referenced by `guest_mem_hdl`
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
    guest_mem_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr_mut(&mut *ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_guest_memory(&*ctx, guest_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };

    match mgr.snapshot_state(guest_mem) {
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
    guest_mem_hdl: Handle,
    layout_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_guest_memory(&*ctx, guest_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    let ret_val = match mgr.get_return_value(guest_mem, &layout) {
        Ok(v) => v,
        Err(e) => return (*ctx).register_err(e),
    };
    register_i32(&mut *ctx, ret_val)
}

/// Sets `addr` to the correct offset in the memory referenced by
/// `guest_mem` to indicate the address of the outb pointer.
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
    guest_mem_hdl: Handle,
    layout_hdl: Handle,
    addr: u64,
) -> Handle {
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let guest_mem = match get_guest_memory_mut(&mut *ctx, guest_mem_hdl) {
        Ok(g) => g,
        Err(e) => return (*ctx).register_err(e),
    };
    let layout = match get_mem_layout(&*ctx, layout_hdl) {
        Ok(l) => l,
        Err(e) => return (*ctx).register_err(e),
    };
    match mgr.set_outb_address(guest_mem, &layout, addr) {
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
    validate_context!(ctx);
    let mgr = match get_mem_mgr(&*ctx, mgr_hdl) {
        Ok(m) => m,
        Err(e) => return (*ctx).register_err(e),
    };
    let val = mgr.get_address_offset(source_addr);
    register_u64(&mut *ctx, val)
}
