use super::{bool::register_boolean, c_func::CFunc, mem_mgr::register_mem_mgr};
use super::{context::Context, sandbox_compat::Sandbox};
use super::{handle::Handle, sandbox_compat::SandboxImpls};
use crate::sandbox_run_options::SandboxRunOptions;
use crate::strings::get_string;
use hyperlight_host::log_then_return;
use hyperlight_host::sandbox;
use hyperlight_host::sandbox::uninitialized::GuestBinary;
use hyperlight_host::sandbox::SandboxConfiguration;
use hyperlight_host::sandbox_state::transition::Noop;
use hyperlight_host::Result;
use hyperlight_host::{mem::ptr::RawPtr, sandbox_state::sandbox::EvolvableSandbox};
use hyperlight_host::{
    sandbox::is_hypervisor_present as check_hypervisor,
    sandbox::is_supported_platform as check_platform,
};
use std::ffi::c_int;
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
    cfg: SandboxConfiguration,
    sandbox_run_options: u32,
    print_output_handler: Option<extern "C" fn(*const c_char) -> c_int>,
) -> Handle {
    CFunc::new("sandbox_new", ctx)
        .and_then_mut(|ctx, _| {
            let bin_path = get_string(ctx, bin_path_hdl)?;
            let run_opts = SandboxRunOptions::from_bits_truncate(sandbox_run_options);
            let should_recycle = run_opts.should_recycle();

            // The print_output_handler is a callback function that is passed in from the client and is used to print output from the guest.
            // we need to wrap that in a closure that we can pass to the sandbox so that we can call it from the sandbox.
            //
            // We only need a closure to be used as a host print handler if we have been passed
            // a non-null pointer to a function as a callback in print_output_handler.
            //
            // We could try and resolve this by a match on print_output_handler returning Option<closure>
            // but creating an option over a closure results in a bunch of issues around lifetimes ,type coercion and borrowing
            // so instead we always create the closure and put the match logic inside it to just return 0 if we have no callback.
            //
            // This logic (the None arm) will never get executed as we match again on the print_output_handler in the UnititializedSandbox constructor
            // below and set the parameter for the host_print_writer closure to None in the case where we dont have a callback.
            //
            // In other words this closure does not get used if we dont have a callback func passed in print_output_handler.

            let callback_writer_func = move |s: String| -> Result<i32> {
                match print_output_handler {
                    Some(f) => {
                        let c_str = std::ffi::CString::new(s)?;
                        let res = f(c_str.as_ptr());
                        Ok(res)
                    }
                    None => Ok(0),
                }
            };

            // We need to box the closure and store it in the Sandbox struct that is placed in the context so it has a long enough lifetime.
            // This is necessary as the value passed to the UnititializedSandbox constructor is a reference to the callback_writer_func
            // which ordnarily will go out of scope at the end of this function.
            // We only do this if we have a callback function to call as we dont use the callback_writer_func closure if were not passed a callback func (see comment above).

            let boxed_callback_writer_func: Option<Box<dyn Fn(String) -> Result<i32>>> =
                match print_output_handler {
                    Some(_) => Some(Box::new(callback_writer_func)),
                    None => None,
                };

            let core_run_opts = run_opts.try_into()?;

            let mut sbox = match print_output_handler {
                Some(_) => {
                    let callback_writer_func = Arc::new(Mutex::new(callback_writer_func));
                    sandbox::UninitializedSandbox::new(
                        GuestBinary::FilePath(bin_path.to_string()),
                        Some(cfg),
                        Some(core_run_opts),
                        Some(&callback_writer_func),
                    )
                }
                None => sandbox::UninitializedSandbox::new(
                    GuestBinary::FilePath(bin_path.to_string()),
                    Some(cfg),
                    Some(core_run_opts),
                    None,
                ),
            }?;
            sbox.set_is_csharp();

            Ok(
                Sandbox::from_uninit(should_recycle, sbox, boxed_callback_writer_func)
                    .register(ctx),
            )
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
            Sandbox::evolve(ctx, sbox_hdl, |should_reuse, u_sbox| {
                if should_reuse {
                    let mu_sbox: hyperlight_host::MultiUseSandbox<'_> =
                        u_sbox.evolve(Noop::default())?;
                    Ok(SandboxImpls::InitMultiUse(Box::new(mu_sbox)))
                } else {
                    let su_sbox: hyperlight_host::SingleUseSandbox<'_> =
                        u_sbox.evolve(Noop::default())?;
                    Ok(SandboxImpls::InitSingleUse(Box::new(su_sbox)))
                }
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

/// get a reference to a `SandboxConfiguration` stored in `ctx`
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
            let mem_mgr = sbox.to_uninit()?.get_mem_mgr_ref();
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
                log_then_return!("String is null ptr");
            }
            let c_str = std::ffi::CStr::from_ptr(msg);
            let msg = c_str.to_str()?;
            let sbox = Sandbox::get_mut(ctx, sbox_hdl)?;
            let rsbox = sbox.to_uninit_mut()?;
            rsbox
                .get_host_funcs()
                .lock()?
                .host_print(String::from(msg))?;
            Ok(Handle::new_empty())
        })
        .ok_or_err_hdl()
}
