use hyperlight_host::hypervisor::handlers::MemAccessHandlerCaller;
use hyperlight_host::{new_error, Result};

use crate::context::Context;
use crate::handle::Handle;
use crate::hdl::Hdl;

/// A FFI-friendly implementation of a `MemAccessHandler`. This type stores
/// a standard C function pointer -- an `extern "C" fn()` -- and implements
/// the `MemAccessHandler`'s `call` method by simply calling the underlying
/// function.
#[derive(Clone)]
pub(crate) struct MemAccessHandlerWrapper {
    func: extern "C" fn(),
}

impl MemAccessHandlerCaller for MemAccessHandlerWrapper {
    fn call(&mut self) -> Result<()> {
        (self.func)();

        Ok(())
    }
}

// TODO: Remove this once this is used.
#[allow(unused)]
/// Get a MemAccessHandlerFunc from the specified handle
pub(crate) fn get_mem_access_handler_func(
    ctx: &Context,
    hdl: Handle,
) -> Result<&MemAccessHandlerWrapper> {
    Context::get(hdl, &ctx.mem_access_handler_funcs, |h| {
        matches!(h, Hdl::MemAccessHandlerFunc(_))
    })
}

/// Get a mutable MemAccessHandlerFunc from the specified handle
pub(crate) fn get_mut_mem_access_handler_func(
    ctx: &mut Context,
    hdl: Handle,
) -> Result<&mut MemAccessHandlerWrapper> {
    Context::get_mut(hdl, &mut ctx.mem_access_handler_funcs, |h| {
        matches!(h, Hdl::MemAccessHandlerFunc(_))
    })
}

/// Create a new memory access function handler from an MemAccessHandlerFn
/// and return a new `Handle` referencing it.
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_access_handler_create(
    ctx: *mut Context,
    cb_ptr: Option<extern "C" fn()>,
) -> Handle {
    let ptr = match cb_ptr {
        Some(ptr) => ptr,
        None => {
            let err = new_error!("invalid mem access handler callback");
            return (*ctx).register_err(err);
        }
    };

    let mem_access_func = MemAccessHandlerWrapper { func: ptr };
    let coll = &mut (*ctx).mem_access_handler_funcs;
    Context::register(mem_access_func, coll, Hdl::MemAccessHandlerFunc)
}

/// Call the memory access function referenced by `mem_access_fn_hdl`
/// and return an empty `Handle` on success, and a `Handle` describing
/// an error otherwise
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn mem_access_handler_call(
    ctx: *mut Context,
    mem_access_fn_hdl: Handle,
) -> Handle {
    let handler = match get_mut_mem_access_handler_func(&mut *ctx, mem_access_fn_hdl) {
        Ok(h) => h,
        Err(e) => return (*ctx).register_err(e),
    };

    match (*handler).call() {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}
