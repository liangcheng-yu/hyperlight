use super::{bool::register_boolean, c_func::CFunc, mem_mgr::register_mem_mgr};
use super::{context::Context, sandbox_compat::Sandbox};
use super::{handle::Handle, sandbox_compat::EitherImpl};
use crate::sandbox::host_funcs::default_writer_func;
use crate::sandbox::mem_mgr::MemMgr;
use crate::{capi::strings::get_string, mem::config::SandboxMemoryConfiguration};
use crate::{func::host::Function1, sandbox};
use crate::{mem::ptr::RawPtr, sandbox_state::sandbox::EvolvableSandbox};
use crate::{sandbox::host_funcs::CallHostPrint, sandbox_state::transition::Noop};
use crate::{
    sandbox::is_hypervisor_present as check_hypervisor,
    sandbox::is_supported_platform as check_platform, SandboxRunOptions,
};
use anyhow::{bail, Result};
use std::os::raw::c_char;
use std::sync::{Arc, Mutex};

/// Create a new `Sandbox` with the given guest binary to execute
/// and return a `Handle` reference to it.
///
/// # Safety
///
/// This function creates new memory, and it is the caller's responsibility
/// to free that memory after it's no longer needed (but no sooner).
///
/// Use only the `handle_free` to do so. Any other method will lead to
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sandbox_new(
    ctx: *mut Context,
    bin_path_hdl: Handle,
    // Why is this not a handle?
    //
    // This struct is created once by the client, passed here, and then
    // never used again by the client and only stored in Rust structures.
    //
    // Further, it's small enough to allow a copy from the caller's stack
    // frame to this function's stack frame, rather than going through all
    // the heap allocation and `Handle` mechanics.
    mem_cfg: SandboxMemoryConfiguration,
    sandbox_run_options: u32,
    print_output_handler: Option<extern "C" fn(*const c_char)>,
) -> Handle {
    CFunc::new("sandbox_new", ctx)
        .and_then_mut(|ctx, _| {
            let bin_path = get_string(ctx, bin_path_hdl)?;
            let sandbox_run_options =
                Some(SandboxRunOptions::from_bits_truncate(sandbox_run_options));

            let writer_func = Arc::new(Mutex::new(move |s: String| -> Result<()> {
                match print_output_handler {
                    Some(f) => {
                        let c_str = std::ffi::CString::new(s)?;
                        f(c_str.as_ptr());
                        Ok(())
                    }
                    None => default_writer_func(s),
                }
            }));

            let mut sbox = sandbox::UninitializedSandbox::new(
                bin_path.to_string(),
                Some(mem_cfg),
                sandbox_run_options,
                None,
            )?;
            writer_func.register(&mut sbox, "writer_func")?;
            Ok(Sandbox::from(sbox).register(ctx))
        })
        .ok_or_err_hdl()
}

/// Calls the initialize method on the `UninitializedSandbox` referenced
/// by `sbox_hdl` in `ctx`, then replaces that `UninitializedSandbox`
/// with the newly-initialized `Sandbox`. The caller can continue to use
/// the same `Handle` for subsequent calls to the Hyperlight C APIs, but
/// if an `UninitializedSandbox` is expected, those calls will now fail.
///
/// # Safety
///
/// The caller must pass a `ctx` to this function that was created by
/// `context_new`, not currently in use by any other function, and not yet
/// freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn sandbox_initialize(ctx: *mut Context, sbox_hdl: Handle) -> Handle {
    CFunc::new("sandbox_initialize", ctx)
        .and_then_mut(|ctx, _| {
            Sandbox::replace(ctx, sbox_hdl, |old| {
                let uninit = match old {
                    EitherImpl::Uninit(u) => u,
                    _ => bail!(
                        "sandbox_initialize: expected an uninitialized sandbox but didn't get one"
                    ),
                };
                let newly_init = uninit.evolve(Noop::default())?;
                Ok(EitherImpl::Init(Box::new(newly_init)))
            })?;
            Ok(Handle::new_empty())
        })
        .ok_or_err_hdl()
}

/// Check the previously-generated stack guard against the value of
/// the stack guard in memory, then return a `Handle` referencing a new
/// boolean in `ctx` indicating whether the stack guard matched or not
/// (`true` is good, `false`, is not). If an error occurred checking the
/// stack guard, return instead a `Handle` referencing an error in `ctx`.
///
/// TODO: remove this after Sandbox is completely rewritten in Rust
///
/// # Safety
///
/// The caller must pass a `ctx` to this function that was created by
/// `context_new`, not currently in use by any other function, and not yet
/// freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn sandbox_check_stack_guard(ctx: *mut Context, sbox_hdl: Handle) -> Handle {
    CFunc::new("sandbox_initialize", ctx)
        .and_then_mut(|ctx, _| {
            let sbox = Sandbox::get(ctx, sbox_hdl)?;
            let check_res: bool = sbox.check_stack_guard()?;
            Ok(register_boolean(ctx, check_res))
        })
        .ok_or_err_hdl()
}

/// Call the entrypoint inside the `Sandbox` referenced by `sbox_hdl`
///
/// # Safety
///
/// The called must pass a `ctx` to this function that was created
/// by `context_new`, not currently in use in any other function,
/// and not yet freed by `context_free`.
#[no_mangle]
pub unsafe extern "C" fn sandbox_call_entry_point(
    ctx: *mut Context,
    sbox_hdl: Handle,
    peb_address: u64,
    seed: u64,
    page_size: u32,
) -> Handle {
    CFunc::new("sandbox_call_entry_point", ctx)
        .and_then_mut(|ctx, _| {
            let sbox = Sandbox::get(ctx, sbox_hdl)?;
            sbox.to_uninit()?
                .call_entry_point(RawPtr::from(peb_address), seed, page_size)?;
            Ok(Handle::new_empty())
        })
        .ok_or_err_hdl()
}

#[no_mangle]
/// Checks if the current platform is supported by Hyperlight.
pub extern "C" fn is_supported_platform() -> bool {
    check_platform()
}

#[no_mangle]
/// Checks if a Hypervisor supported by Hyperlight is available.
pub extern "C" fn is_hypervisor_present() -> bool {
    check_hypervisor()
}

/// get a reference to a `SandboxMemoryConfiguration` stored in `ctx`
/// and pointed to by `handle`.
///
/// TODO: this is temporary until we have a complete C API for the Sandbox
///
/// # Safety
///
/// The caller must pass a `ctx` to this function that was created
/// by `context_new`, not currently in use in any other function,
/// and not yet freed by `context_free` and a valid handle to a `Sandbox` that is assocaited with the Context and has not been freed.
///
#[no_mangle]
pub unsafe extern "C" fn sandbox_get_memory_mgr(ctx: *mut Context, sbox_hdl: Handle) -> Handle {
    CFunc::new("sandbox_get_memory_mgr", ctx)
        .and_then_mut(|ctx, _| {
            let sbox = Sandbox::get(ctx, sbox_hdl)?;
            let mem_mgr = sbox.to_uninit()?.get_mem_mgr();
            Ok(register_mem_mgr(ctx, mem_mgr.clone()))
        })
        .ok_or_err_hdl()
}

/// Call host_print function on a sandbox pointed to by `handle` stored in `ctx`
///
/// TODO: this should be removed once we have a complete C API for the Sandbox - it only exists for testing
///
/// # Safety
///
/// The caller must pass a `ctx` to this function that was created
/// by `context_new`, not currently in use in any other function,
/// and not yet freed by `context_free` and a valid handle to a `Sandbox` that is assocaited with the Context and has not been freed.
///
#[no_mangle]
pub unsafe extern "C" fn sandbox_call_host_print(
    ctx: *mut Context,
    sbox_hdl: Handle,
    msg: *const c_char,
) -> Handle {
    CFunc::new("sandbox_call_host_print", ctx)
        .and_then_mut(|ctx, _| {
            if msg.is_null() {
                bail!("String is null ptr");
            }
            let c_str = std::ffi::CStr::from_ptr(msg);
            let msg = c_str.to_str()?;
            let sbox = Sandbox::get_mut(ctx, sbox_hdl)?;
            let rsbox = sbox.to_uninit_mut()?;
            rsbox.host_print(String::from(msg))?;
            Ok(Handle::new_empty())
        })
        .ok_or_err_hdl()
}
