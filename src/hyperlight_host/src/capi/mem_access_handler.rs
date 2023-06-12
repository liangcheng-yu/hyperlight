use crate::capi::context::Context;
use crate::capi::handle::Handle;
use crate::capi::hdl::Hdl;
use anyhow::Result;

/// A wrapper around a standard C function pointer that represents a
/// memory access, commonly used by hypervisor implementations.
#[derive(Clone)]
pub struct MemAccessHandlerWrapper {
    func: extern "C" fn(),
}

impl MemAccessHandlerWrapper {
    /// Call the wrapped handler function
    pub(crate) fn call(&self) {
        (self.func)()
    }
}

/// Create a new `MemAccessHandlerWrapper` with the given `func`
#[cfg(test)]
#[cfg(target_os = "linux")]
pub(crate) fn new_mem_access_handler_wrapper(func: extern "C" fn()) -> MemAccessHandlerWrapper {
    MemAccessHandlerWrapper { func }
}

/// Get a MemAccessHandlerFunc from the specified handle
pub(crate) fn get_mem_access_handler_func(
    ctx: &Context,
    hdl: Handle,
) -> Result<&MemAccessHandlerWrapper> {
    Context::get(hdl, &ctx.mem_access_handler_funcs, |h| {
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
            let err = anyhow::Error::msg("invalid mem access handler callback");
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
    let handler = match get_mem_access_handler_func(&*ctx, mem_access_fn_hdl) {
        Ok(h) => h,
        Err(e) => return (*ctx).register_err(e),
    };
    (*handler).call();
    Handle::new_empty()
}
