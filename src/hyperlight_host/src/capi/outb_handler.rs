use crate::capi::handle::Handle;
use crate::capi::hdl::Hdl;
use crate::{capi::context::Context, hypervisor::handlers::OutBHandlerCaller};
use anyhow::Result;

/// A FFI-friendly implementation of a `OutBHandler`. This type stores
/// a standard C function pointer -- an `extern "C" fn(u16, u64)` -- and
/// implements the `OutBHandler`'s `call` method by simply calling the
/// underlying function.
#[derive(Clone)]
pub(crate) struct OutBHandlerWrapper {
    func: extern "C" fn(u16, u64),
}

impl OutBHandlerCaller for OutBHandlerWrapper {
    fn call(&mut self, port: u16, payload: u64) -> anyhow::Result<()> {
        (self.func)(port, payload);
        Ok(())
    }
}

/// Get a OutbHandlerFunc from the specified handle
pub(crate) fn get_outb_handler_func(ctx: &Context, hdl: Handle) -> Result<&OutBHandlerWrapper> {
    Context::get(hdl, &ctx.outb_handler_funcs, |h| {
        matches!(h, Hdl::OutbHandlerFunc(_))
    })
}

/// Get a mutable OutbHandlerFunc from the specified handle
pub(crate) fn get_outb_handler_func_mut(
    ctx: &mut Context,
    hdl: Handle,
) -> Result<&mut OutBHandlerWrapper> {
    Context::get_mut(hdl, &mut ctx.outb_handler_funcs, |h| {
        matches!(h, Hdl::OutbHandlerFunc(_))
    })
}

/// Create a new outb function handler from an OutbHandlerFunc
/// and return a new `Handle` referencing it.
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
///
/// You must also call this function with a Function Pointer to  a function that:
///
/// points to a valid callback function that takes a u16 and a u64 and returns a bool
/// and that is valid for at least as long as the last point at which
/// you call the function via `outb_fn_handler_call`.
#[no_mangle]
pub unsafe extern "C" fn outb_fn_handler_create(
    ctx: *mut Context,
    cb_ptr: Option<extern "C" fn(u16, u64)>,
) -> Handle {
    let ptr = match cb_ptr {
        Some(ptr) => ptr,
        None => {
            let err = anyhow::Error::msg("invalid outb handler callback");
            return (*ctx).register_err(err);
        }
    };

    let outb_func = OutBHandlerWrapper { func: ptr };
    let coll = &mut (*ctx).outb_handler_funcs;
    Context::register(outb_func, coll, Hdl::OutbHandlerFunc)
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
pub unsafe extern "C" fn outb_fn_handler_call(
    ctx: *mut Context,
    outb_fn_handler_ref: Handle,
    port: u16,
    payload: u8,
) -> Handle {
    let handler = match get_outb_handler_func_mut(&mut *ctx, outb_fn_handler_ref) {
        Ok(h) => h,
        Err(e) => return (*ctx).register_err(e),
    };

    match (*handler).call(port, payload as u64) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}
