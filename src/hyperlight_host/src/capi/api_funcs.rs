use super::handle::Handle;
use super::hdl::Hdl;
use super::strings::{to_string, RawCString};
/// C-compatible functions for dealing with functions that are called
/// across the VM boundary. For example, the APIs herein are used
/// to register functions implemented by the host but called by
/// the guest, or implemented by the guest but called by the host.
use super::{c_func::CFunc, context::Context};
use crate::func::def::HostFunc;
use crate::validate_context;
use anyhow::Result;

mod impls {
    use super::super::hdl::Hdl;
    use super::super::sandbox::{get_sandbox, get_sandbox_mut};
    use crate::capi::context::Context;
    use crate::capi::handle::Handle;
    use crate::capi::val_ref::get_val;
    use anyhow::{anyhow, Result};

    /// Call the guest function in `ctx` referenced by `func_name` with
    /// the parameter in `ctx` referenced by `param_hdl`. The function
    /// will be called using the sandbox in `ctx` referenced by
    /// `sbox_hdl`.
    ///
    /// If the sandbox, function, and parameter are all valid in `ctx`,
    /// the return value of the function will be stored in `ctx` and
    /// referenced by `param_hdl`.
    pub fn call_guest_func(
        ctx: &mut Context,
        sbox_hdl: Handle,
        func_name: &str,
        param_hdl: Handle,
    ) -> Result<Handle> {
        let sbox = get_sandbox(ctx, sbox_hdl)?;
        let param = get_val(ctx, param_hdl)?;
        let ret = sbox
            .call_guest_func(func_name.to_string(), param)
            .map_err(|e| anyhow!(e.message))?;
        Ok(Context::register(ret, &mut ctx.vals, Hdl::Val))
    }

    /// Register the function in `ctx` referenced by `func_hdl` on the
    /// sandbox in `ctx` referenced by `sbox_hdl` to the name `func_name`.
    pub fn add_host_func(
        ctx: &mut Context,
        sbox_hdl: Handle,
        func_name: String,
        func_hdl: Handle,
    ) -> Result<Handle> {
        let func_rg = super::get_host_func(ctx, func_hdl)?;

        let func = func_rg.to_owned();
        let sbox = get_sandbox_mut(ctx, sbox_hdl)?;
        sbox.register_host_func(func_name, func);
        Ok(Handle::from(Hdl::Empty()))
    }

    /// Call the function in `ctx` called `func_name` on the sandbox
    /// in `ctx` referenced by `sbox_hdl`, passing the parameter in
    /// `ctx` referenced by `param_hdl`.
    ///
    /// If the call was a success, `Ok` will be returned and the
    /// function's return value will be stored in `ctx` and
    /// referenced by the returned `Handle`.
    pub fn call_host_func(
        ctx: &mut Context,
        sbox_hdl: Handle,
        func_name: &str,
        arg_hdl: Handle,
    ) -> Result<Handle> {
        let sbox = get_sandbox(ctx, sbox_hdl)?;
        let arg = get_val(ctx, arg_hdl)?;
        match sbox.host_funcs.get(func_name) {
            Some(func) => {
                let ret = func.call(arg);
                Ok(Context::register(*ret, &mut ctx.vals, Hdl::Val))
            }
            None => Err(anyhow!("no such host function {}", func_name)),
        }
    }
}

/// Get the `HostFunc` stored in `ctx` and referenced by `handle`.
fn get_host_func(ctx: &Context, handle: Handle) -> Result<&HostFunc> {
    Context::get(handle, &ctx.host_funcs, |h| matches!(h, Hdl::HostFunc(_)))
}
/// Call a function on the guest.
///
/// # Safety
///
/// You are responsible for freeing the memory this function
/// creates. Make sure you call `handle_free` exactly once
/// with the returned value after you're done with it.
#[no_mangle]
pub unsafe extern "C" fn guest_func_call(
    ctx: *mut Context,
    sbox_hdl: Handle,
    name: RawCString,
    param_hdl: Handle,
) -> Handle {
    validate_context!(ctx);

    let func_name = to_string(name);
    match impls::call_guest_func(&mut (*ctx), sbox_hdl, &func_name, param_hdl) {
        Ok(hdl) => hdl,
        Err(e) => (*ctx).register_err(e),
    }
}

/// Add a host-implemented function to be available on the guest.
///
/// Note that you must have created a new host function with
/// `create_host_func` prior to this call. The return value of that
/// function can be passed to this function as `func_hdl`.
///
/// # Safety
///
/// This function does not take ownership of name, but
/// instead copies it. It does, however, take ownership
/// of func, so it's important that you do not interact
/// with func in any way after passing it to this function.
///
/// The return value from this function is either an empty
/// `Handle` or a `Handle` that references an error. In either
/// case, you must call `handle_free` exactly once after
/// you're done with it. Failure to do this will result
/// in a memory leak or invalid memory access
/// (i.e. a use-after-free)
#[no_mangle]
pub unsafe extern "C" fn host_func_register(
    ctx: *mut Context,
    sbox_hdl: Handle,
    func_name: RawCString,
    func_hdl: Handle,
) -> Handle {
    validate_context!(ctx);

    let func_name_str = to_string(func_name);
    match impls::add_host_func(&mut (*ctx), sbox_hdl, func_name_str, func_hdl) {
        Ok(hdl) => hdl,
        Err(e) => (*ctx).register_err(e),
    }
}

/// Call the host function in sbox with the given name
/// and arguments.
///
/// If such a function exists with the given name and it was
/// successfully called, returns the result and leaves err
/// empty. Otherwise, returns a blank Val and fills in an
/// error message.
///
/// # Safety
///
/// This function should, in the common case, not be called.
/// The internals of Sandbox will automatically call functions
/// when the guest calls them (via the outb passing mechanism).
/// It is provided primarily for debugging purposes.
///
/// TODO: mark this as debug-only
#[no_mangle]
pub unsafe extern "C" fn host_func_call(
    ctx: *mut Context,
    sbox_hdl: Handle,
    name: RawCString,
    arg_hdl: Handle,
) -> Handle {
    CFunc::new("host_func_call", ctx)
        .and_then_mut(|c, _| {
            let func_name = to_string(name);
            impls::call_host_func(c, sbox_hdl, &func_name, arg_hdl)
        })
        .ok_or_err_hdl()
}
