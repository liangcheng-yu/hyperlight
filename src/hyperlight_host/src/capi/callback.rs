use super::context::Context;
use super::handle::Handle;
use crate::func::args::Val;
use crate::func::def::HostFunc;

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
pub struct Callback {
    pub func: extern "C" fn(*mut Val) -> *mut Val,
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
/// The memory pointed to by `cb` must live at least as long as
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
    callback: &'static Callback,
) -> Handle {
    let func = |val: &Val| -> Box<Val> {
        let func = callback.func;
        let bx_param = Box::new(val.clone());
        Box::from_raw(func(Box::into_raw(bx_param)))
    };

    let hf = HostFunc::new(Box::new(func));
    (*ctx).register_host_func(hf)
}
