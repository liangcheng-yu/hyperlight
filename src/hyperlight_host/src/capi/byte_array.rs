use super::context::Context;
use super::fill_vec;
use super::handle::Handle;
use super::strings::{to_string, RawCString};
use anyhow::anyhow;

mod impls {
    use super::super::context::Context;
    use super::super::handle::Handle;
    use anyhow::{anyhow, bail, Result};
    use std::fs::read;

    pub fn get(ctx: &mut Context, handle: Handle) -> Result<Vec<u8>> {
        match ctx.remove_byte_array(handle) {
            Some(bytes) => Ok(bytes),
            None => bail!("handle is not a byte array handle"),
        }
    }

    pub fn len(ctx: &Context, handle: Handle) -> Result<usize> {
        let arr = ctx.get_byte_array(handle)?;
        Ok(arr.len())
    }

    pub fn new_from_file(file_name: &str) -> Result<Vec<u8>> {
        // this line will cause valgrind version 3.15 to fail due to
        // a syscall being passed 0x0. it's a known issue:
        // https://github.com/rust-lang/rust/issues/68979. you must have
        // a more modern version of valgrind that doesn't consider
        // the NUL byte an issue. I've tested (and documented) with
        // 3.19 and all works fine there.
        read(file_name).map_err(|e| anyhow!("Error reading file {}: {}", file_name, e))
    }
}

/// Copy all the memory from `arr_ptr` to `arr_ptr + arr_len` into a new
/// byte array, register the new byte array's memory with the given `ctx`,
/// and return a `Handle` that references it.
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
    let vec = fill_vec(arr_ptr, arr_len);
    (*ctx).register_byte_array(vec)
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
    let file_name_str = to_string(file_name);
    let vec_res = impls::new_from_file(&file_name_str);
    match vec_res {
        Ok(vec) => (*ctx).register_byte_array(vec),
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
    match impls::len(&(*ctx), handle) {
        Ok(l) => l as i64,
        Err(_) => -1,
    }
}

/// Get the byte array referenced by `handle`, return a pointer to the
/// underlying array, and remove it from `ctx`.
///
/// The length of the memory referenced by the returned pointer is
/// equal to the value returned from `byte_array_len(ctx, handle)`.
///
/// If no such byte array exists for the given `handle`, `NULL`
/// will be returned.
///
///
/// # Safety
///
/// The caller is responsible for the memory referenced by the returned
/// pointer. After this function returns, the caller must therefore free
/// this memory when they're done with it, and it will no longer exist
/// in `ctx`.
#[no_mangle]
pub unsafe extern "C" fn byte_array_get(ctx: *mut Context, handle: Handle) -> *const u8 {
    match impls::get(&mut *ctx, handle) {
        Ok(vec) => {
            let ptr = vec.as_ptr();
            std::mem::forget(vec);
            ptr
        }
        Err(_) => std::ptr::null(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::context::Context;
    use super::super::err::handle_is_error;
    use super::impls;
    use anyhow::Result;
    #[test]
    fn byte_array_new_from_file() -> Result<()> {
        let filenames = vec!["./testdata/simpleguest.exe", "./testdata/callbackguest.exe"];
        for filename in filenames {
            let file = impls::new_from_file(filename)?;
            assert!(!file.is_empty())
        }

        Ok(())
    }

    #[test]
    fn byte_array_len() -> Result<()> {
        let ctx = Context::default();
        let barr = vec![1, 2, 3];
        let barr_len = barr.len();
        let barr_hdl = ctx.register_byte_array(barr);
        assert!(!handle_is_error(barr_hdl));
        assert_eq!(impls::len(&ctx, barr_hdl)?, barr_len);

        Ok(())
    }

    #[test]
    fn byte_array_get() -> Result<()> {
        let mut ctx = Context::default();
        let barr = vec![1, 2, 3];
        let barr_copy = barr.clone();
        let barr_hdl = ctx.register_byte_array(barr);
        assert!(!handle_is_error(barr_hdl));

        {
            let ret_barr = impls::get(&mut ctx, barr_hdl)?;
            assert_eq!(barr_copy, ret_barr);
        }

        {
            let ret_barr_res = impls::get(&mut ctx, barr_hdl);
            assert!(ret_barr_res.is_err());
        }

        Ok(())
    }
}
