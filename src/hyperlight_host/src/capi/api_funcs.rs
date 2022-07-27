use super::context::Context;
use super::handle::Handle;
use super::strings::{to_string, RawCString};

mod impls {
    use super::super::hdl::Hdl;
    use crate::capi::context::Context;
    use crate::capi::handle::Handle;
    use anyhow::{anyhow, Result};
    pub fn call_guest_func(
        ctx: &mut Context,
        sbox_hdl: Handle,
        func_name: &str,
        param_hdl: Handle,
    ) -> Result<Handle> {
        let sbox = ctx.get_sandbox(sbox_hdl)?;
        let param = ctx.get_val(param_hdl)?;
        let ret = sbox
            .call_guest_func(func_name.to_string(), &param)
            .map_err(|e| anyhow!(e.message))?;
        Ok(ctx.register_val(ret))
    }

    pub fn add_host_func(
        ctx: &mut Context,
        sbox_hdl: Handle,
        func_name: String,
        func_hdl: Handle,
    ) -> Result<Handle> {
        let mut sbox = ctx.get_sandbox_mut(sbox_hdl)?;
        let func_rg = ctx.get_host_func(func_hdl)?;

        sbox.register_host_func(func_name, func_rg.to_owned());
        Ok(Handle::from(Hdl::Empty()))
    }

    pub fn call_host_func(
        ctx: &mut Context,
        sbox_hdl: Handle,
        func_name: &str,
        arg_hdl: Handle,
    ) -> Result<Handle> {
        let sbox = ctx.get_sandbox(sbox_hdl)?;
        let arg = ctx.get_val(arg_hdl)?;
        match sbox.host_funcs.get(func_name) {
            Some(func) => {
                let ret = func.call(&arg);
                Ok(ctx.register_val(*ret))
            }
            None => Err(anyhow!("no such host function {}", func_name)),
        }
    }
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
    let func_name = to_string(name);
    match impls::call_host_func(&mut (*ctx), sbox_hdl, &func_name, arg_hdl) {
        Ok(hdl) => hdl,
        Err(e) => (*ctx).register_err(e),
    }
}
