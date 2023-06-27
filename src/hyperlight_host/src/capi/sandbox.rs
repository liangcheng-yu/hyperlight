use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::{c_func::CFunc, mem_mgr::register_mem_mgr};
use crate::mem::ptr::RawPtr;
use crate::{
    capi::strings::get_string, mem::config::SandboxMemoryConfiguration,
    sandbox::HostFunctionWithOneArg, sandbox::UnintializedSandbox as RustSandbox,
};
use crate::{
    sandbox::is_hypervisor_present as check_hypervisor,
    sandbox::is_supported_platform as check_platform, sandbox_run_options::SandboxRunOptions,
};
use anyhow::{bail, Result};
use std::cell::RefCell;
use std::os::raw::c_char;
use std::rc::Rc;

/// This is the C API for the `Sandbox` type.
pub struct Sandbox {
    rust_sandbox: RustSandbox<'static>,
}

impl Sandbox {
    fn get_rust_sandbox(&self) -> &RustSandbox<'static> {
        &self.rust_sandbox
    }

    fn get_rust_sandbox_mut(&mut self) -> &mut RustSandbox<'static> {
        &mut self.rust_sandbox
    }
}

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
pub unsafe extern "C" fn sandbox_new(
    ctx: *mut Context,
    bin_path_hdl: Handle,
    // TODO: Why is this not a handle , I derived this from load_guest_binary which took the struct rather than a handle to it?
    // In the orignal code it just passed the struct and did not validate it.
    // However ,I dont see why we cant just pass the struct here and not a handle to it as it is allocated in the client used once (i.e. we dont ever use it again in a C API call)
    // and since its copied (both implement copy) then it doesnt matter if the client frees it after the call.
    mem_cfg: Option<&mut SandboxMemoryConfiguration>,
    sandbox_run_options: u32,
    print_output_handler: Option<extern "C" fn(*const c_char)>,
) -> Handle {
    CFunc::new("sandbox_new", ctx)
        .and_then_mut(|ctx, _| {
            let bin_path = get_string(ctx, bin_path_hdl)?;
            let mem_cfg: Option<SandboxMemoryConfiguration> = mem_cfg.map(|cfg| (*cfg));
            let sandbox_run_options =
                Some(SandboxRunOptions::from_bits_truncate(sandbox_run_options));

            let writer_func = print_output_handler.map(|f| HostFunctionWithOneArg {
                func: Rc::new(RefCell::new(move |s: String| -> Result<()> {
                    let c_str = std::ffi::CString::new(s)?;
                    f(c_str.as_ptr());
                    Ok(())
                })) as Rc<RefCell<dyn FnMut(String) -> Result<()>>>,
            });
            let sbox = RustSandbox::new(
                bin_path.to_string(),
                mem_cfg,
                writer_func,
                sandbox_run_options,
            )?;

            Ok(register_sandbox(ctx, Sandbox { rust_sandbox: sbox }))
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
            let sbox = get_sandbox(ctx, sbox_hdl)?;
            sbox.get_rust_sandbox()
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
/// Checks if the current platform is supported by Hyperlight.
pub extern "C" fn is_hypervisor_present() -> bool {
    check_hypervisor()
}

/// Get a read-only reference to a `Sandbox` stored in `ctx` and
/// pointed to by `handle`.
fn get_sandbox(ctx: &Context, handle: Handle) -> Result<&Sandbox> {
    Context::get(handle, &ctx.sandboxes, |s| matches!(s, Hdl::Sandbox(_)))
}

/// Get a mutable reference to a `Sandbox` stored in `ctx` and
/// pointed to by `handle`.
fn get_sandbox_mut(ctx: &mut Context, hdl: Handle) -> Result<&mut Sandbox> {
    Context::get_mut(hdl, &mut ctx.sandboxes, |h| matches!(h, Hdl::Sandbox(_)))
}

fn register_sandbox(ctx: &mut Context, val: Sandbox) -> Handle {
    Context::register(val, &mut ctx.sandboxes, Hdl::Sandbox)
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
            let sbox = get_sandbox(ctx, sbox_hdl)?;
            let mem_mgr = sbox.get_rust_sandbox().get_mem_mgr();
            Ok(register_mem_mgr(ctx, mem_mgr))
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
            let sbox = get_sandbox_mut(ctx, sbox_hdl)?;
            let rsbox = sbox.get_rust_sandbox_mut();
            rsbox.host_print(String::from(msg))?;
            Ok(Handle::new_empty())
        })
        .ok_or_err_hdl()
}
