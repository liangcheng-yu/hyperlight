use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::{c_func::CFunc, mem_mgr::register_mem_mgr};
use crate::{capi::strings::get_string, mem::config::SandboxMemoryConfiguration, sandbox::Sandbox};
use crate::{
    sandbox::is_hypervisor_present as check_hypervisor,
    sandbox::is_supported_platform as check_platform,
};
use anyhow::Result;

/// Create a new `Sandbox` with the given guest binary to execute
/// and return a `Handle` reference to it.
///
/// # Safety
///
/// This function creates new memory on the heap, and it
/// is the caller's responsibility to free that memory when
/// it's no longer needed (but no sooner). Use `handle_free`
/// to do so.
#[no_mangle]
pub unsafe extern "C" fn sandbox_new(ctx: *mut Context, bin_path_hdl: Handle) -> Handle {
    CFunc::new("sandbox_new", ctx)
        .and_then_mut(|ctx, _| {
            let bin_path = get_string(ctx, bin_path_hdl)?;
            let sbox = Sandbox::new(bin_path.to_string());
            Ok(register_sandbox(ctx, sbox))
        })
        .ok_or_err_hdl()
}

/// Fetch the string from `ctx` referenced by `bin_path_hdl` and use
/// that as the name of the file to load as a guest binary. Load that
/// file as a binary according to the configuration in `mem_cfg` and the
/// other two `bool` parameters, then return a new `Handle` referencing
/// a new `SandboxMemoryManager` in `ctx` to manage that loaded binary.
///
/// # Safety
///
/// `ctx` must be created by `context_new`, owned by the caller, and
/// not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn sandbox_load_guest_binary(
    ctx: *mut Context,
    mem_cfg: SandboxMemoryConfiguration,
    bin_path_hdl: Handle,
    run_from_process_memory: bool,
    run_from_guest_binary: bool,
) -> Handle {
    CFunc::new("sandbox_load_guest_binary", ctx)
        .and_then_mut(|ctx, _| {
            let bin_path_str = get_string(ctx, bin_path_hdl)?;
            let mem_mgr = Sandbox::load_guest_binary(
                mem_cfg,
                bin_path_str.as_str(),
                run_from_process_memory,
                run_from_guest_binary,
            )?;
            Ok(register_mem_mgr(ctx, mem_mgr))
        })
        .ok_or_err_hdl()
}

#[no_mangle]
/// Checks if the current platform is supported by Hyperlight.
pub extern "C" fn is_supported_platform() -> bool {
    check_platform()
}

#[no_mangle]
/// Checks if the current platform is supported by Hyperlight.
pub extern "C" fn is_hypervisor_present() -> bool {
    check_hypervisor()
}

/// Get a read-only reference to a `Sandbox` stored in `ctx` and
/// pointed to by `handle`.
pub fn get_sandbox(ctx: &Context, handle: Handle) -> Result<&Sandbox> {
    Context::get(handle, &ctx.sandboxes, |s| matches!(s, Hdl::Sandbox(_)))
}

fn register_sandbox(ctx: &mut Context, val: Sandbox) -> Handle {
    Context::register(val, &mut ctx.sandboxes, Hdl::Sandbox)
}

/// Get a read-and-write capable reference to a `Sandbox` stored in
/// `ctx` and pointed to by `handle`.
pub fn get_sandbox_mut(ctx: &mut Context, handle: Handle) -> Result<&mut Sandbox> {
    Context::get_mut(handle, &mut ctx.sandboxes, |s| matches!(s, Hdl::Sandbox(_)))
}
