use super::handle::Handle;
use super::hdl::Hdl;
use super::strings::{to_string, RawCString};
use super::{arrays::raw_vec::RawVec, context::Context};
use crate::{validate_context, validate_context_or_panic};
use anyhow::Result;
use anyhow::{anyhow, Error};

mod impls {
    use super::super::context::Context;
    use super::super::handle::Handle;
    use anyhow::{anyhow, bail, Result};
    use std::fs::read;

    /// Returns a reference to the byte array
    pub(crate) fn get(ctx: &Context, handle: Handle) -> Result<&Vec<u8>> {
        match super::get_byte_array(ctx, handle) {
            Ok(bytes) => Ok(bytes),
            _ => bail!("handle is not a byte array handle"),
        }
    }

    /// Returns the length of the byte array.
    pub(crate) fn len(ctx: &Context, handle: Handle) -> Result<usize> {
        let arr = super::get_byte_array(ctx, handle)?;
        Ok(arr.len())
    }

    pub(crate) fn new_from_file(file_name: &str) -> Result<Vec<u8>> {
        // this line will cause valgrind version 3.15 to fail due to
        // a syscall being passed 0x0. it's a known issue:
        // https://github.com/rust-lang/rust/issues/68979. you must have
        // a more modern version of valgrind that doesn't consider
        // the NUL byte an issue. I've tested (and documented) with
        // 3.19 and all works fine there.
        read(file_name).map_err(|e| anyhow!("Error reading file {}: {}", file_name, e))
    }
}

/// Get the byte array in `ctx` referenced by `handle` in a `ReadResult`
/// that can be used only to read the bytes, or `Err` if the
/// byte array could not be fetched from `ctx`.
pub(crate) fn get_byte_array(ctx: &Context, handle: Handle) -> Result<&Vec<u8>> {
    Context::get(handle, &ctx.byte_arrays, |b| matches!(b, Hdl::ByteArray(_)))
}

/// Copy all the memory in the range
/// `[ arr_ptr, arr_ptr + (arr_len * sizeof(u8)) )`
/// (noting the right side of the range is exclusive) into a
/// new byte array, register it with the given `ctx`, and return a new
/// `Handle` referencing it.
///
/// # Safety
///
/// `arr_ptr` must point to a valid, owned, contiguous memory region
/// of `arr_len` `i8` values. The caller is responsible for ensuring
/// this memory is not modified in any way, or deleted, while this
/// function is executing. Additionally, this memory is borrowed,
/// so it is the caller's responsibility to ensure that it is freed
/// after this function returns.
#[no_mangle]
pub unsafe extern "C" fn byte_array_new(
    ctx: *mut Context,
    arr_ptr: *const u8,
    arr_len: usize,
) -> Handle {
    validate_context!(ctx);

    if arr_ptr.is_null() {
        let err = Error::msg("array pointer passed to byte_array_new is NULL");
        return (*ctx).register_err(err);
    }
    let raw_vec = RawVec::copy_from_ptr(arr_ptr as *mut u8, arr_len);
    Context::register(raw_vec.into(), &mut (*ctx).byte_arrays, Hdl::ByteArray)
}

/// Read the entire contents of the file at `file_name` into
/// memory, then create a new byte array with the contents and
/// return a reference to that byte array.
///
/// # Safety
///
/// The given context `ctx` must be valid and not modified
/// or deleted at any time while this function is executing.
/// Additionally, `file_name` must be a valid, null-terminated
/// C-style string and not modified or deleted at any time
/// during this function's execution.
#[no_mangle]
pub unsafe extern "C" fn byte_array_new_from_file(
    ctx: *mut Context,
    file_name: RawCString,
) -> Handle {
    validate_context!(ctx);

    let file_name_str = to_string(file_name);
    let vec_res = impls::new_from_file(&file_name_str);
    match vec_res {
        Ok(vec) => Context::register(vec, &mut (*ctx).byte_arrays, Hdl::ByteArray),
        Err(err) => (*ctx).register_err(anyhow!(err)),
    }
}

/// Return the length of the byte array referenced by `handle`.
///
/// If no byte array is referenced by `handle`, return `-1`.
///
/// # Safety
///
/// `ctx` must refer to an existing `Context` the caller owns, and
/// that context must not be modified or deleted at any time during
/// the execution of this function.
#[no_mangle]
pub unsafe extern "C" fn byte_array_len(ctx: *const Context, handle: Handle) -> i64 {
    validate_context_or_panic!(ctx);

    match impls::len(&(*ctx), handle) {
        Ok(l) => l as i64,
        Err(_) => -1,
    }
}

/// Get the byte array referenced by `handle`, copy the backing memory,
/// and return a pointer to the copy, transfering ownership of that memory
/// to the caller.
///
/// The length of the memory referenced by the returned pointer is
/// equal to the value returned from `byte_array_len(ctx, handle)`.
///
/// If no such byte array exists for the given `handle`, `NULL`
/// will be returned.
///
/// # Safety
///
/// `ctx` must refer to an existing `Context` the caller owns, and
/// that context must not be modified or deleted at any time during
/// the execution of this function.
///
/// The caller is responsible for the memory referenced by the returned
/// pointer. After this function returns, the caller must therefore free
/// this memory when they're done with it by calling `byte_array_raw_free`
/// and passing this pointer and the length of the byte array as returned
/// by `byte_array_len`.
///
/// **It is not guaranteed that all memory will be correctly freed if you
/// call any other function than `byte_array_raw_free`**.
///
/// The Context is still responsible for the byte array memory after this function returns.
#[no_mangle]
pub unsafe extern "C" fn byte_array_get_raw(ctx: *mut Context, handle: Handle) -> *mut u8 {
    validate_context_or_panic!(ctx);

    match impls::get(&*ctx, handle) {
        Ok(vec) => {
            // copy the vec and move it into a RawVec,
            // then convert that RawVec to a pointer.
            let raw_vec = RawVec::from(vec.clone());
            let (ptr, _): (*mut u8, usize) = raw_vec.into();
            ptr
        }
        Err(e) => {
            (*ctx).register_err(e);
            std::ptr::null_mut()
        }
    }
}

/// Free the byte array's memory associated with `ptr` based on its length.
///
/// # Safety
///
/// You must only call this function exactly once per `ByteArray`, and only
/// call it after you're done using `ptr`.
#[no_mangle]
pub unsafe extern "C" fn byte_array_raw_free(ptr: *mut u8, len: usize) -> bool {
    RawVec::from_ptr(ptr, len);
    true
}

#[cfg(test)]
mod tests {
    use super::super::context::Context;
    use super::super::handle_status::{handle_get_status, HandleStatus};
    use super::super::hdl::Hdl;
    use super::impls;
    use crate::testing::{callback_guest_path, simple_guest_path};
    use anyhow::Result;
    #[test]
    fn byte_array_new_from_file() {
        let filenames = vec![simple_guest_path().unwrap(), callback_guest_path().unwrap()];
        for filename in filenames {
            let file = impls::new_from_file(&filename).unwrap();
            assert!(!file.is_empty())
        }
    }

    #[test]
    fn byte_array_len() -> Result<()> {
        let mut ctx = Context::default();
        let barr = vec![1, 2, 3];
        let barr_len = barr.len();
        let barr_hdl = Context::register(barr, &mut ctx.byte_arrays, Hdl::ByteArray);
        assert_eq!(handle_get_status(barr_hdl), HandleStatus::ValidOther);
        assert_eq!(impls::len(&ctx, barr_hdl)?, barr_len);

        Ok(())
    }

    #[test]
    fn byte_array_get_raw() -> Result<()> {
        let mut ctx = Context::default();
        let barr = vec![1, 2, 3];
        let barr_copy = barr.clone();
        let barr_hdl = Context::register(barr, &mut ctx.byte_arrays, Hdl::ByteArray);
        assert_eq!(handle_get_status(barr_hdl), HandleStatus::ValidOther);

        {
            let ret_barr = impls::get(&ctx, barr_hdl)?;
            assert_eq!(barr_copy, ret_barr.as_slice());
        }

        Ok(())
    }
}
