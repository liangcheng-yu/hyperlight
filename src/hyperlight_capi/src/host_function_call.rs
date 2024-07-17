use std::mem;

use hyperlight_common::flatbuffer_wrappers::function_call::FunctionCall;
use hyperlight_host::{HyperlightError, Result};

use super::arrays::raw_vec::RawVec;
use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use crate::validate_context_or_panic;

/// Return true if the given handle `hdl` references a `FunctionCall` representing a Host Function Call in `ctx`,
/// and false otherwise
///
/// # Safety
///
/// `ctx` must be a valid pointer to a `Context` created with `context_new`,
/// owned by you, and not yet freed with `context_free`
#[no_mangle]
pub unsafe extern "C" fn handle_is_host_function_call(ctx: *const Context, hdl: Handle) -> bool {
    validate_context_or_panic!(ctx);
    get_host_function_call(&*ctx, hdl).is_ok()
}

/// Return the value of the `FunctionCall` in `ctx` referenced by `hdl` as ptr.
/// The ptr references a `flatbuffer` serialistion of a `FunctionCall`.
/// If `hdl` does not reference a `FunctionCall` in `ctx`, the return value is
/// NULL.
///
/// # Safety
///
/// `ctx` must be a valid pointer to a `Context` created with `context_new`,
/// owned by you, and not yet freed with `context_free`
///
/// The caller is responsible for the memory referenced by the returned
/// pointer. After this function returns, the caller must therefore free
/// this memory when they're done with it by calling `guest_error_raw_free`
/// and passing this pointer.
///
/// **It is not guaranteed that all memory will be correctly freed if you
/// call any other function than `host_function_call_flatbuffer_free`**.
///
/// The Context is still responsible for the byte array memory after this function returns.
#[no_mangle]
pub unsafe extern "C" fn handle_get_host_function_call_flatbuffer(
    ctx: *mut Context,
    hdl: Handle,
) -> *mut u8 {
    validate_context_or_panic!(ctx);

    match get_host_function_call(&*ctx, hdl) {
        Ok(host_function_call) => {
            match Vec::try_from(host_function_call.clone()) {
                Ok(fb_bytes) => {
                    // Move the fb_bytes vec into a RawVec, then return the
                    // pointer to that underlying RawVec.
                    //
                    // This means that the memory must be freed by the caller
                    // invoking `host_function_call_flatbuffer_free`.
                    //
                    // The returned Vec is a size prefixed flatbuffer, which
                    // means the first 4 bytes are the size of the buffer
                    // and the capacity of the Vec is the same as the size of
                    // the buffer + 4 bytes for the size field.
                    // therefore `host_function_call_flatbuffer_free` can safely
                    // reconstruct the Vec, bring it back into a RawVec, and
                    // then drop it.
                    let raw_vec = RawVec::from(fb_bytes);
                    let (ptr, _): (*mut u8, _) = raw_vec.into();
                    ptr
                }
                Err(_) => {
                    (*ctx).register_err(HyperlightError::Error(
                        "Failed to serialise host function call".to_string(),
                    ));
                    std::ptr::null_mut()
                }
            }
        }
        Err(_) => {
            //TODO: Update when we have a GetLastErrorFunction
            (*ctx).register_err(HyperlightError::Error(
                "Failed to get host function call".to_string(),
            ));
            std::ptr::null_mut()
        }
    }
}

/// Free the memory associated with the `FunctionCall`s `ptr`.
///
/// # Safety
///
/// You must only call this function exactly once per `ptr' returned from `handle_get_host_function_call_flatbuffer`, and only
/// call it after you're done using `ptr`. The pointer must be a valid pointer to a `FunctionCall` serialised using `flatbuffers`.
#[no_mangle]
pub unsafe extern "C" fn host_function_call_flatbuffer_free(ptr: *mut u8) -> bool {
    // the size of the memory is placed in the first 4 bytes of the memory for a size prefixed flatbuffer
    // the size does not include the size of the size field, so we need to add 4 to the size
    // the capacity of the Vec is the same as the size of the buffer
    let len = std::ptr::read(ptr as *const u32) as usize + mem::size_of::<u32>();
    let raw_vec = RawVec::from_ptr(ptr, len);
    drop(raw_vec);
    true
}

fn get_host_function_call(ctx: &Context, hdl: Handle) -> Result<&FunctionCall> {
    Context::get(hdl, &ctx.host_function_calls, |hdl| {
        matches!(hdl, Hdl::HostFunctionCall(_))
    })
}
