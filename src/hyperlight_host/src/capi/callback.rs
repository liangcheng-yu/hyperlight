use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use crate::func::args::Val;
use crate::func::def::HostFunc;
use anyhow::Error;

/// Callback is a wrapper around a standard C
/// function pointer.
///
/// The hyperlight_host library does not pass
/// C function pointers directly due to the
/// way that Rust's lifetimes interact with
/// the (static) memory pointed to by an extern
/// function pointer. Therefore, it is
/// necessary to wrap extern function pointers
/// in a simple struct to sidestep the issue.
///
/// You will most likely pass a new `Callback` to
/// `create_host_func`. If you do that, you'll need
/// to make sure that the `Callback` lives at least
/// as long as the `HostFunc` with which it was created.
///
/// In most cases, you'll add the newly created
/// `HostFunc` to a `Sandbox`; that means you'll need
/// to make sure the `Callback` lives at least
/// as long as the `Sandbox`.
///
/// For more information on generally how
/// extern function pointers work in Rust, see
/// documentation:
///
/// https://rust-lang.github.io/unsafe-code-guidelines/layout/function-pointers.html
///
/// # Safety
///
/// The `Val*` parameter the callback receives points to new memory that
/// it owns. It must free that memory with `val_ref_free` before it returns,
/// or there will be a leak.
#[repr(C)]
#[derive(Debug)]
pub struct Callback {
    /// The function pointer to the callback.
    ///
    /// This field must be a C-compatible function pointer.
    /// This is an Option as it is possible the pointer could be null
    /// 
    /// From https://rust-lang.github.io/unsafe-code-guidelines/layout/function-pointers.html
    /// However, null values are not supported by the Rust function pointer types -- just like references, the expectation is that you use Option to create nullable pointers. `Option<fn(Args...) -> Ret> ` will have the exact same ABI as `fn(Args...) -> Ret`,but additionally allows null pointer values.
    pub func: Option<extern "C" fn(*mut Val) -> *mut Val>,
}

/// Create a new `HostFunc` from a `Callback`.
///
/// Return values from this function can be used in
/// functions such as `add_host_func`. If the callback you
/// pass to this function creates new `Val`s, they must be
/// created with `new_val_ref` or `empty_val_ref`.
///
/// # Safety
///
/// `callback` must:
/// - not be `NULL`
/// - point to memory that lives at least until immediately after
/// you intend to call the `HostFunc` (e.g. via `call_host_func`).
///
/// Most often, you'll create a `Callback`, a `HostFunc` immediately
/// thereafter from the `Callback`, and then you'll pass the `HostFunc`
/// to `add_host_func`. At that point, the `Sandbox` will take ownership
/// of the `HostFunc`, and it will be destroyed when the `Sandbox` is
/// destroyed.
///
/// To call a `HostFunc` returned by this function,
/// use `call_host_func`. You must free the memory returned
/// by this function using `free_host_func`.
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
/// // creating the HostFunc from a callback is different from
/// // registering it with the sandbox.
/// Handle host_func_ref = host_func_create(ctx, my_callback);
/// Handle host_func_reg = host_func_register(ctx, sbox, host_func_name, host_func_ref);
/// // note: my_val_ref is a value created with val_ref_new() elsewhere.
/// Handle call_ref = host_func_call(ctx, sbox, host_func_name, my_val_ref);
///
/// handle_free(call_ref);
/// handle_free(host_func_ref);
/// handle_free(host_func_reg);
/// context_free(ctx);
/// ```
#[no_mangle]
pub unsafe extern "C" fn host_func_create(
    ctx: *mut Context,
    callback: Option<&'static Callback>,
) -> Handle {
    // Note about the callback parameter:
    //
    // It's not a good idea to accept a reference in an extern "C"
    // function. It would be better to use a pointer instead.
    // But, we're using the reference here because we need a static
    // reference and I (arschles) don't know a good way to do something
    // similar to static references with pointers.
    //
    // This function's parameter list in C is:
    //
    // (Context* ctx, Callback* callback)
    //
    // So, what happens if someone passes NULL for callback?
    //
    // According to https://rust-lang.github.io/unsafe-code-guidelines/layout/function-pointers.html:
    // null values are not supported by the Rust function pointer types -- just like references, the expectation is that you use Option to create nullable pointers. `Option<fn(Args...) -> Ret> ` will have the exact same ABI as `fn(Args...) -> Ret`,but additionally allows null pointer values.
    // then we can test for None to check for null pointer.
    let cb = match callback {
        Some(c) => c,
        None => return (*ctx).register_err(Error::msg("NULL callback")),
    };

    let cbfunc = match cb.func {
        Some(c) => c,
        None => return (*ctx).register_err(Error::msg("NULL callback func")),
    };

    let func  = move |val: &Val| -> Box<Val> {
        let func = cbfunc;
        let bx_param = Box::new(val.clone());
        Box::from_raw(func(Box::into_raw(bx_param)))
    };

    let hf = HostFunc::new(Box::new(func));
    Context::register(hf, &(*ctx).host_funcs, Hdl::HostFunc)
}
