use super::guest_mem::{get_guest_memory, get_guest_memory_mut};
use super::mem_layout::get_mem_layout;
use super::{byte_array::get_byte_array, context::Context, handle::Handle, hdl::Hdl};
use crate::mem::{config::SandboxMemoryConfiguration, mgr::SandboxMemoryManager};
use crate::validate_context;
use anyhow::{anyhow, Result};

fn get_mem_mgr(ctx: &Context, hdl: Handle) -> Result<&SandboxMemoryManager> {
    Context::get(hdl, &ctx.mem_mgrs, |h| matches!(h, Hdl::MemMgr(_))).map_err(|e| anyhow!(e))
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
