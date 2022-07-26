use super::context::Context;
use super::handle::Handle;
use crate::mem::pe::PEInfo;

mod impls {
    use super::super::context::Context;
    use super::super::handle::Handle;
    use crate::mem::pe::PEInfo;
    use anyhow::Result;

    pub fn pe_relocate(
        ctx: &mut Context,
        pe_info_hdl: Handle,
        payload_hdl: Handle,
        addr_to_load_at: usize,
    ) -> Result<Handle> {
        let pe_info = ctx.get_pe_info(pe_info_hdl)?;
        let mut bar = ctx.get_byte_array_mut(payload_hdl)?;
        let reloc_patches = pe_info.get_exe_relocation_patches(addr_to_load_at, bar.as_slice())?;
        pe_info.apply_relocation_patches(&reloc_patches, bar.as_mut_slice())?;
        Ok(payload_hdl)
    }

    pub fn pe_get<T, U: FnOnce(&PEInfo) -> Result<T>>(
        ctx: &Context,
        pe_hdl: Handle,
        get_fn: U,
    ) -> Result<T> {
        let pe = ctx.get_pe_info(pe_hdl)?;
        get_fn(&pe)
    }

    pub fn pe_parse(ctx: &Context, bytes_handle: Handle) -> Result<PEInfo> {
        let bytes = ctx.get_byte_array(bytes_handle)?;
        PEInfo::new(&bytes)
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
        Ok(hdl) => (*ctx).register_pe_info(hdl),
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
    impls::pe_get(&*ctx, pe_handle, PEInfo::stack_reserve).unwrap_or(0)
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
    impls::pe_get(&*ctx, pe_handle, PEInfo::stack_commit).unwrap_or(0)
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
    impls::pe_get(&*ctx, pe_handle, PEInfo::heap_reserve).unwrap_or(0)
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
    impls::pe_get(&*ctx, pe_handle, PEInfo::heap_commit).unwrap_or(0)
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
    impls::pe_get(&*ctx, pe_handle, PEInfo::try_entry_point_offset).unwrap_or(0)
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
    use super::super::context::Context;
    use super::super::err::handle_is_error;
    use crate::mem::pe::PEInfo;
    use anyhow::Result;
    use std::fs;

    const PE_FILE_NAMES: [&str; 1] = ["./testdata/simpleguest.exe"];

    #[test]
    fn pe_getters() -> Result<()> {
        for pe_file_name in PE_FILE_NAMES {
            let ctx = Context::default();
            let pe_file_bytes = fs::read(pe_file_name)?;
            let pe_info = PEInfo::new(pe_file_bytes.as_slice())?;
            let pe_file_bytes_hdl = ctx.register_byte_array(pe_file_bytes);
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
            let payload_hdl = ctx.register_byte_array(pe_file_bytes);
            assert!(!handle_is_error(payload_hdl));
            let addr = 123;
            let pe_info_hdl = {
                let pe_info = super::impls::pe_parse(&ctx, payload_hdl)?;
                ctx.register_pe_info(pe_info)
            };
            assert!(!handle_is_error(pe_info_hdl));
            let res_hdl = super::impls::pe_relocate(&mut ctx, pe_info_hdl, payload_hdl, addr)?;
            assert!(!handle_is_error(res_hdl));

            assert_eq!(payload_hdl, res_hdl);
            // TODO: assert that the payload is changed.
        }

        Ok(())
    }
}
