use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use crate::mem::pe::PEInfo;

mod impls {
    use crate::capi::context::Context;
    use crate::capi::handle::Handle;
    use crate::capi::hdl::Hdl;
    use crate::mem::pe::PEInfo;
    use anyhow::Result;

    pub fn pe_relocate(
        ctx: &mut Context,
        pe_info_hdl: Handle,
        payload_hdl: Handle,
        addr_to_load_at: usize,
    ) -> Result<Handle> {
        let pe_info = Context::get(pe_info_hdl, &ctx.pe_infos, |p| matches!(p, Hdl::PEInfo(_)))?;
        let bar = Context::get_mut(payload_hdl, &mut ctx.byte_arrays, |b| {
            matches!(b, Hdl::ByteArray(_))
        })?;
        let reloc_patches = pe_info.get_exe_relocation_patches(addr_to_load_at, bar.as_slice())?;
        pe_info.apply_relocation_patches(&reloc_patches, bar.as_mut_slice())?;
        Ok(payload_hdl)
    }

    pub fn get_pe_and<T, U: FnOnce(&PEInfo) -> Result<T>>(
        ctx: &Context,
        pe_hdl: Handle,
        get_fn: U,
    ) -> Result<T> {
        let pe = Context::get(pe_hdl, &ctx.pe_infos, |p| matches!(p, Hdl::PEInfo(_)))?;
        get_fn(pe)
    }

    pub fn pe_parse(ctx: &Context, bytes_handle: Handle) -> Result<PEInfo> {
        let bytes = Context::get(bytes_handle, &ctx.byte_arrays, |p| {
            matches!(p, Hdl::ByteArray(_))
        })?;
        PEInfo::new(bytes)
    }
}

/// Parse a PE file from a byte array.
///
/// # Safety
///
/// `ctx` must be memory created by `context_new`, owned by the caller,
/// and not modified or deleted while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn pe_parse(ctx: *mut Context, byte_array_handle: Handle) -> Handle {
    match impls::pe_parse(&*ctx, byte_array_handle) {
        Ok(pe_info) => Context::register(pe_info, &mut (*ctx).pe_infos, Hdl::PEInfo),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get the stack reserve value from the PE file referenced by `pe_handle`,
/// or `0` if `pe_handle` does not reference a valid PE file or there
/// was another problem fetching the value.
///
/// # Safety
///
/// `ctx` must be memory created by `context_new`, owned by the caller,
/// and not modified or deleted while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn pe_stack_reserve(ctx: *mut Context, pe_handle: Handle) -> u64 {
    impls::get_pe_and(&*ctx, pe_handle, PEInfo::stack_reserve).unwrap_or(0)
}

/// Get the stack commit value from the PE file referenced by `pe_handle`,
/// or `0` if `pe_handle` does not reference a valid PE file or there
/// was another problem fetching the value.
///
/// # Safety
///
/// `ctx` must be memory created by `context_new`, owned by the caller,
/// and not modified or deleted while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn pe_stack_commit(ctx: *mut Context, pe_handle: Handle) -> u64 {
    impls::get_pe_and(&*ctx, pe_handle, PEInfo::stack_commit).unwrap_or(0)
}

/// Get the heap reserve value from the PE file referenced by `pe_handle`,
/// or `0` if `pe_handle` does not reference a valid PE file or there
/// was another problem fetching the value.
///
/// # Safety
///
/// `ctx` must be memory created by `context_new`, owned by the caller,
/// and not modified or deleted while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn pe_heap_reserve(ctx: *mut Context, pe_handle: Handle) -> u64 {
    impls::get_pe_and(&*ctx, pe_handle, PEInfo::heap_reserve).unwrap_or(0)
}

/// Get the heap commit value from the PE file referenced by `pe_handle`,
/// or `0` if `pe_handle` does not reference a valid PE file or there
/// was another problem fetching the value.
///
/// # Safety
///
/// `ctx` must be memory created by `context_new`, owned by the caller,
/// and not modified or deleted while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn pe_heap_commit(ctx: *mut Context, pe_handle: Handle) -> u64 {
    impls::get_pe_and(&*ctx, pe_handle, PEInfo::heap_commit).unwrap_or(0)
}

/// Get the entry point offset value from the PE file referenced by `pe_handle`,
/// or `0` if `pe_handle` does not reference a valid PE file or there
/// was another problem fetching the value.
///
/// # Safety
///
/// `ctx` must be memory created by `context_new`, owned by the caller,
/// and not modified or deleted while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn pe_entry_point_offset(ctx: *mut Context, pe_handle: Handle) -> u64 {
    impls::get_pe_and(&*ctx, pe_handle, PEInfo::try_entry_point_offset).unwrap_or(0)
}

/// Apply relocations to the payload referenced by `payload_hdl`
/// based on its relocation table.
///
/// It is expected that `payload_hdl` references a byte array
/// created with `byte_array_new`.
///
/// On success, the payload will be updated in-place and the same
/// `payload_hdl` will be returned. On failure, a `Handle`
/// referencing an error will be returned. In both cases,
/// new memory will be created that should be freed with
/// `handle_free`.
///
/// # Safety
///
/// `ctx` must be memory created by `context_new`, owned by the caller,
/// and not modified or deleted while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn pe_relocate(
    ctx: *mut Context,
    pe_handle: Handle,
    byte_array_handle: Handle,
    addr_to_load_at: usize,
) -> Handle {
    match impls::pe_relocate(&mut (*ctx), pe_handle, byte_array_handle, addr_to_load_at) {
        Ok(hdl) => hdl,
        Err(e) => (*ctx).register_err(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::capi::context::Context;
    use crate::capi::handle_status::{handle_get_status, HandleStatus};
    use crate::capi::hdl::Hdl;
    use crate::mem::pe::PEInfo;
    use anyhow::Result;
    use std::fs;

    const PE_FILE_NAMES: [&str; 1] = ["./testdata/simpleguest.exe"];

    #[test]
    fn pe_getters() -> Result<()> {
        for pe_file_name in PE_FILE_NAMES {
            let mut ctx = Context::default();
            let pe_file_bytes = fs::read(pe_file_name)?;
            let pe_info = PEInfo::new(pe_file_bytes.as_slice())?;
            let pe_file_bytes_hdl =
                Context::register(pe_file_bytes, &mut ctx.byte_arrays, Hdl::ByteArray);
            let pe_info_ret = super::impls::pe_parse(&ctx, pe_file_bytes_hdl)?;

            assert_eq!(pe_info.stack_commit()?, pe_info_ret.stack_commit()?);
        }

        Ok(())
    }

    #[test]
    fn pe_relocate() -> Result<()> {
        for pe_file_name in PE_FILE_NAMES {
            let pe_file_bytes = fs::read(pe_file_name)?;
            let mut ctx = Context::default();
            let payload_hdl =
                Context::register(pe_file_bytes, &mut ctx.byte_arrays, Hdl::ByteArray);
            assert_eq!(handle_get_status(payload_hdl), HandleStatus::ValidOther);
            let addr = 123;
            let pe_info_hdl = {
                let pe_info = super::impls::pe_parse(&ctx, payload_hdl)?;
                Context::register(pe_info, &mut ctx.pe_infos, Hdl::PEInfo)
            };
            assert_eq!(handle_get_status(pe_info_hdl), HandleStatus::ValidOther);
            let res_hdl = super::impls::pe_relocate(&mut ctx, pe_info_hdl, payload_hdl, addr)?;
            assert_eq!(handle_get_status(res_hdl), HandleStatus::ValidOther);

            assert_eq!(payload_hdl, res_hdl);
            // TODO: assert that the payload is changed.
        }

        Ok(())
    }
}
