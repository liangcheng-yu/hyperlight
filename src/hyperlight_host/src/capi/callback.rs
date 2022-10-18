use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use crate::func::args::Val;
use crate::func::def::HostFunc;

/// Create a new `HostFunc` from a C function pointer
///
/// Return values from this function can be used in
/// functions such as `add_host_func`. If the callback you
/// pass to this function creates new `Val`s, they must be
/// created with `new_val_ref` or `empty_val_ref`.
///
/// # Safety
///
/// If `callback` is non-`NULL`, it must be function pointer that
/// points to code that lives in memory for at least as long as you
/// intend to hold the `Handle` returned by this function.
/// It's best practice to have this pointer point to a function
/// that lives for the duration of your program.
////
/// To call a `HostFunc` returned by this function,
/// use `call_host_func`. You must free the memory returned
/// by this function using `handle_free`.
///
/// # Example
///
/// ```c
/// // note: assume my_callback is a function
/// // you've defined elsewhere
/// const char* host_func_name = "test_func1";
/// Context* ctx = context_new();
/// Handle sbox_ref = sandbox_new(ctx, "sample_binary");
///
/// // creating the HostFunc from a function pointer is different from
/// // registering it with the sandbox.
/// Handle host_func_ref = host_func_create(ctx, my_callback);
/// Handle host_func_reg = host_func_register(
///     ctx,
///     sbox,
///     host_func_name,
///     host_func_ref
/// );
/// // note: my_val_ref is a value created with val_ref_new() elsewhere.
/// Handle call_ref = host_func_call(ctx, sbox, host_func_name, my_val_ref);
///
/// handle_free(ctx, call_ref);
/// handle_free(ctx, host_func_ref);
/// handle_free(ctx, host_func_reg);
/// context_free(ctx);
/// ```
#[no_mangle]
pub unsafe extern "C" fn host_func_create(
    ctx: *mut Context,
    cb_opt: Option<extern "C" fn(*mut Val) -> *mut Val>,
) -> Handle {
    let cb = match cb_opt {
        Some(cb) => cb,
        None => return (*ctx).register_err_msg("NULL callback func"),
    };
    // a note about the cb parameter:
    //
    // cb is a HostFuncPtr, which is an extern "C" fn ...
    // in other words, it's a function pointer.
    //
    // according to the unsafe Rust guide on function pointers
    // (https://rust-lang.github.io/unsafe-code-guidelines/layout/function-pointers.html)
    // such a function pointer has no lifetime of its own and is assumed to
    // point to code with a static lifetime.
    //
    // that means we can use it in the closure that we call 'closure' below

    let closure = move |val: &Val| -> Box<Val> {
        let func = cb;
        let bx_param = Box::new(val.clone());
        Box::from_raw(func(Box::into_raw(bx_param)))
    };

    let hf = HostFunc::new(Box::new(closure));
    Context::register(hf, &(*ctx).host_funcs, Hdl::HostFunc)
}

#[cfg(test)]
mod tests {
    use super::host_func_create;
    use crate::capi::handle::handle_free;
    use crate::capi::{
        context::{context_free, context_new, Context},
        hdl::Hdl,
    };
    use crate::func::args::Val;

    extern "C" fn host_func(_: *mut Val) -> *mut Val {
        std::ptr::null_mut()
    }

    #[test]
    fn test_host_func_create() {
        let ctx = context_new();
        let ctx_deref = unsafe { &(*ctx) };
        let hdl = unsafe { host_func_create(ctx, Some(host_func)) };
        let res = Context::get(hdl, &ctx_deref.errs, |h| matches!(h, Hdl::Err(_)));
        if res.is_ok() {
            panic!("host_func_create returned an error");
        }
        unsafe { handle_free(ctx, hdl) };
        unsafe { context_free(ctx) };
    }
}
