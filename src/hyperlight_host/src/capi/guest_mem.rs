use super::context::{Context, ReadResult, WriteResult};
use super::handle::Handle;
use super::hdl::Hdl;
use crate::mem::guest_mem::GuestMemory;

mod impls {
    use crate::capi::handle::Handle;
    use crate::capi::{byte_array::get_byte_array, context::Context};
    use anyhow::{bail, Result};
    pub fn get_address(ctx: &Context, hdl: Handle) -> Result<usize> {
        let guest_mem = super::get_guest_memory(ctx, hdl)?;
        Ok(guest_mem.base_addr())
    }

    pub fn read_int_64(ctx: &Context, hdl: Handle, addr: u64) -> Result<i64> {
        let guest_mem = super::get_guest_memory(ctx, hdl)?;
        (*guest_mem).read_i64(addr)
    }

    pub fn write_int_64(ctx: &Context, hdl: Handle, addr: usize, val: usize) -> Result<()> {
        let mut guest_mem = super::get_guest_memory_mut(ctx, hdl)?;
        (*guest_mem).write_u64(addr, val as u64)
    }

    pub fn read_int_32(ctx: &Context, hdl: Handle, addr: u64) -> Result<i32> {
        let guest_mem = super::get_guest_memory(ctx, hdl)?;
        (*guest_mem).read_i32(addr)
    }

    pub fn write_int_32(ctx: &Context, hdl: Handle, addr: usize, val: i32) -> Result<()> {
        let mut guest_mem = super::get_guest_memory_mut(ctx, hdl)?;
        (*guest_mem).write_i32(addr, val)?;
        Ok(())
    }

    pub fn copy_byte_array(
        ctx: &Context,
        guest_mem_hdl: Handle,
        byte_array_hdl: Handle,
        address: usize,
        arr_start: usize,
        arr_length: usize,
    ) -> Result<()> {
        let mut guest_mem = super::get_guest_memory_mut(ctx, guest_mem_hdl)?;
        let byte_arr = get_byte_array(ctx, byte_array_hdl)?;
        let byte_arr_len = (*byte_arr).len();
        if arr_start >= byte_arr_len {
            bail!("Array start ({}) is out of bounds", arr_start);
        }
        let arr_end = arr_start + arr_length;
        if arr_end > byte_arr_len {
            bail!("Array end ({}) is out of bounds", arr_end);
        }
        let data = &(*byte_arr)[arr_start..arr_start + arr_length];
        (*guest_mem).copy_into(data, address)
    }
}

/// Get the `GuestMemory` stored in `ctx` and referenced by `hdl` and return
/// it inside a `ReadResult` suitable only for read operations.
///
/// Returns `Ok` if `hdl` is a valid `GuestMemory` in `ctx`,
/// `Err` otherwise.
pub fn get_guest_memory(ctx: &Context, hdl: Handle) -> ReadResult<GuestMemory> {
    Context::get(hdl, &ctx.guest_mems, |g| matches!(g, Hdl::GuestMemory(_)))
}

/// Get the `GuestMemory` stored in `ctx` and referenced by `hdl` and return
/// it inside a `WriteResult` suitable for mutation.
///
/// Returns `Ok` if `hdl` is a valid `GuestMemory` in `ctx`,
/// `Err` otherwise.
pub fn get_guest_memory_mut(ctx: &Context, hdl: Handle) -> WriteResult<GuestMemory> {
    Context::get_mut(hdl, &ctx.guest_mems, |g| matches!(g, Hdl::GuestMemory(_)))
}

/// Create a new instance of guest memory with `min_size` bytes.
///
/// Guest memory is shared memory intended to be shared with a
/// hypervisor partition.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_new(ctx: *mut Context, min_size: u64) -> Handle {
    match GuestMemory::new(min_size as usize) {
        Ok(guest_mem) => Context::register(guest_mem, &(*ctx).guest_mems, Hdl::GuestMemory),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get the starting address of the guest memory referenced
/// by `hdl` in `ctx`, or `0` if the handle is invalid.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_get_address(ctx: *const Context, hdl: Handle) -> usize {
    impls::get_address(&*ctx, hdl).unwrap_or(0)
}

/// Fetch the byte array in `ctx` referenced by `byte_array_hdl`
/// and the guest memory in `ctx` referenced by `guest_mem_hdl`,
/// then copy the data from the byte array in the range
/// `[arr_start, arr_start + arr_length)` (i.e. the left side is
/// inclusive and the right side is not inclusive) into the guest
/// memory starting at address `address`.
///
/// Return an empty `Handle` if both the guest memory and byte array
/// were found and the copy succeeded, and an error handle otherwise.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_copy_byte_array(
    ctx: *mut Context,
    guest_mem_hdl: Handle,
    byte_array_hdl: Handle,
    address: usize,
    arr_start: usize,
    arr_length: usize,
) -> Handle {
    match impls::copy_byte_array(
        &*ctx,
        guest_mem_hdl,
        byte_array_hdl,
        address,
        arr_start,
        arr_length,
    ) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Fetch guest memory from `ctx` referenced by `hdl`, then read
/// a single 64 bit integer from it at address `addr`.
///
/// Return a `Handle` containing the integer if the read succeeded,
/// and an error otherwise.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_read_int_64(
    ctx: *mut Context,
    hdl: Handle,
    addr: u64,
) -> Handle {
    match impls::read_int_64(&*ctx, hdl, addr) {
        Ok(val) => Context::register(val, &(*ctx).int64s, Hdl::Int64),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Write a single 64 bit integer `val` to guest memory in `ctx` referenced
/// by `hdl` at `addr`.
///
/// Return an empty `Handle` if the write succeeded,
/// and an error `Handle` otherwise.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_write_int_64(
    ctx: *mut Context,
    hdl: Handle,
    addr: usize,
    val: usize,
) -> Handle {
    match impls::write_int_64(&*ctx, hdl, addr, val) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Fetch guest memory from `ctx` referenced by `hdl`, then read
/// a single 32 bit integer from it at address `addr`.
///
/// Return a `Handle` containing the integer if the read succeeded,
/// and an error otherwise.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_read_int_32(
    ctx: *mut Context,
    hdl: Handle,
    addr: u64,
) -> Handle {
    match impls::read_int_32(&*ctx, hdl, addr) {
        Ok(val) => Context::register(val, &(*ctx).int32s, Hdl::Int32),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Write a single 32 bit integer `val` to guest memory in `ctx` referenced
/// by `hdl` at `addr`.
///
/// Return an empty `Handle` if the write succeeded,
/// and an error `Handle` otherwise.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_write_int_32(
    ctx: *mut Context,
    hdl: Handle,
    addr: usize,
    val: i32,
) -> Handle {
    match impls::write_int_32(&*ctx, hdl, addr, val) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}
