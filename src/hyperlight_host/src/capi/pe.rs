use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use crate::capi::handle::handle_new_empty;
use crate::{validate_context, validate_context_or_panic};

/// An immutable set of PE File headers.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PEHeaders {
    /// Stack reserve size.
    pub stack_reserve: u64,

    /// Stack commit size.
    pub stack_commit: u64,

    /// Heap reserve size.
    pub heap_reserve: u64,

    /// Heap commit size.
    pub heap_commit: u64,

    /// Entrypoint offset.
    pub entrypoint_offset: u64,

    /// Preferred load address.
    pub preferred_load_address: u64,
}

/// Wrapper around a PEFile that is stored in the context.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PEFile {
    /// Handle for the PE file so that it can be released later.
    pub handle: Handle,

    /// Pointer to a byte array with the PE file contents.
    pub arr_ptr: *const u8,

    /// The size of the byte array.
    pub arr_len: usize,
}

mod impls {
    use crate::capi::context::Context;
    use crate::capi::handle::Handle;
    use crate::capi::hdl::Hdl;
    use crate::mem::pe::pe_info::PEInfo;
    use anyhow::Result;

    use super::PEHeaders;

    /// Updates the PE File contents to relocate it to the specified load address.
    /// If no relocations are required, the payload is not modified.
    pub fn pe_relocate(
        ctx: &mut Context,
        pe_info_hdl: Handle,
        payload_hdl: Handle,
        addr_to_load_at: usize,
    ) -> Result<()> {
        let pe_info = Context::get(pe_info_hdl, &ctx.pe_infos, |p| matches!(p, Hdl::PEInfo(_)))?;
        let payload_bytes = Context::get_mut(payload_hdl, &mut ctx.byte_arrays, |b| {
            matches!(b, Hdl::ByteArray(_))
        })?;
        let payload = payload_bytes.as_mut_slice();
        let reloc_patches = pe_info.get_exe_relocation_patches(payload, addr_to_load_at)?;

        if reloc_patches.is_empty() {
            return Ok(());
        }

        // Modify the payload and apply the patches
        pe_info.apply_relocation_patches(payload, reloc_patches)?;
        Ok(())
    }

    pub fn pe_parse(ctx: &Context, bytes_handle: Handle) -> Result<PEInfo> {
        let bytes = Context::get(bytes_handle, &ctx.byte_arrays, |p| {
            matches!(p, Hdl::ByteArray(_))
        })?;
        PEInfo::new(bytes)
    }

    pub fn pe_get_headers(ctx: &Context, pe_info_hdl: Handle) -> Result<PEHeaders> {
        let pe_info = Context::get(pe_info_hdl, &ctx.pe_infos, |p| matches!(p, Hdl::PEInfo(_)))?;

        let hdrs = PEHeaders {
            entrypoint_offset: pe_info.entry_point_offset(),
            stack_reserve: pe_info.stack_reserve(),
            stack_commit: pe_info.stack_commit(),
            heap_reserve: pe_info.heap_reserve(),
            heap_commit: pe_info.heap_commit(),
            preferred_load_address: pe_info.preferred_load_address(),
        };

        Ok(hdrs)
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
    validate_context!(ctx);

    match impls::pe_parse(&*ctx, byte_array_handle) {
        Ok(pe_info) => Context::register(pe_info, &mut (*ctx).pe_infos, Hdl::PEInfo),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Read the headers for a PE file.
///
/// # Safety
///
/// `ctx` must be memory created by `context_new`, owned by the caller,
/// and not modified or deleted while this function is executing.
#[no_mangle]
pub unsafe extern "C" fn pe_get_headers(ctx: *mut Context, pe_handle: Handle) -> PEHeaders {
    validate_context_or_panic!(ctx);

    impls::pe_get_headers(&*ctx, pe_handle).unwrap()
}

/// Apply relocations to the payload referenced by `payload_hdl`
/// based on its relocation table.
///
/// It is expected that `payload_hdl` references a byte array
/// created with `byte_array_new`.
///
/// On success, the payload will be updated in-place and an empty
/// handle will be returned. On failure, a `Handle`
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
    validate_context!(ctx);

    match impls::pe_relocate(&mut (*ctx), pe_handle, byte_array_handle, addr_to_load_at) {
        Ok(_) => handle_new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::capi::byte_array::{self, byte_array_new};
    use crate::capi::context::Context;
    use crate::capi::handle_status::{handle_get_status, HandleStatus};
    use crate::capi::pe::impls::pe_get_headers;
    use crate::capi::pe::pe_parse;
    use anyhow::Result;
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::fs;
    use std::io::Cursor;
    use std::path::PathBuf;

    struct PEFileTest<'a> {
        path: &'a str,
        stack_size: u64,
        heap_size: u64,
        entrypoint: u64,
        load_address: u64,
        has_relocations: bool,
    }
    const PE_FILES: [PEFileTest; 2] = [
        PEFileTest {
            path: "testdata/simpleguest.exe",
            stack_size: 65536,
            heap_size: 131072,
            entrypoint: 14256,
            load_address: 5368709120,
            has_relocations: true,
        },
        PEFileTest {
            path: "testdata/callbackguest.exe",
            stack_size: 65536,
            heap_size: 131072,
            entrypoint: 4112,
            load_address: 5368709120,
            has_relocations: false,
        },
    ];

    #[test]
    fn pe_headers() -> Result<()> {
        for test in PE_FILES {
            let pe_file_name = test.path;
            let mut pe_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            pe_path.push(pe_file_name);
            let pe_path = pe_path.to_str().unwrap();
            let pe_file_bytes = fs::read(pe_path)
                .unwrap_or_else(|e| panic!("error opening test file {}: {}", pe_path, e));

            let pe_headers = unsafe {
                let ctx = &mut Context::default();
                let payload_hdl = byte_array_new(ctx, pe_file_bytes.as_ptr(), pe_file_bytes.len());
                assert_eq!(handle_get_status(payload_hdl), HandleStatus::ValidOther);

                let pe_hdl = pe_parse(ctx, payload_hdl);
                assert_eq!(handle_get_status(pe_hdl), HandleStatus::ValidOther);

                pe_get_headers(ctx, pe_hdl)?
            };

            // Check that the headers are populated
            assert_eq!(
                test.stack_size, pe_headers.stack_reserve,
                "unexpected stack reserve for {pe_file_name}"
            );
            assert_eq!(
                test.stack_size, pe_headers.stack_commit,
                "unexpected stack commit for {pe_file_name}"
            );
            assert_eq!(
                pe_headers.heap_reserve, test.heap_size,
                "unexpected heap reserve for {pe_file_name}"
            );
            assert_eq!(
                pe_headers.heap_commit, test.heap_size,
                "unexpected heap commit for {pe_file_name}"
            );
            assert_eq!(
                pe_headers.entrypoint_offset, test.entrypoint,
                "unexpected entrypoint for {pe_file_name}"
            );
            assert_eq!(
                pe_headers.preferred_load_address, test.load_address,
                "unexpected load address for {pe_file_name}"
            );
        }

        Ok(())
    }

    #[test]
    fn pe_relocate() {
        for test in PE_FILES {
            let pe_file_name = test.path;
            let mut pe_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            pe_path.push(pe_file_name);
            let pe_path = pe_path.to_str().unwrap();
            let pe_file_bytes = fs::read(pe_path)
                .unwrap_or_else(|e| panic!("error opening test file {}: {}", pe_path, e));

            unsafe {
                let ctx = &mut Context::default();
                let payload_hdl = byte_array_new(ctx, pe_file_bytes.as_ptr(), pe_file_bytes.len());
                assert_eq!(
                    handle_get_status(payload_hdl),
                    HandleStatus::ValidOther,
                    "failed to load {pe_file_name} into a byte array"
                );

                let pe_hdl = pe_parse(ctx, payload_hdl);
                assert_eq!(
                    handle_get_status(pe_hdl),
                    HandleStatus::ValidOther,
                    "failed to parse {pe_file_name}"
                );

                let addr = 0x20000;
                let reloc_result = super::pe_relocate(ctx, pe_hdl, payload_hdl, addr);
                assert_eq!(
                    handle_get_status(reloc_result),
                    HandleStatus::ValidEmpty,
                    "failed to relocate {pe_file_name}"
                );

                let relocated_payload_len = byte_array::byte_array_len(ctx, payload_hdl) as usize;
                assert_eq!(
                    relocated_payload_len,
                    pe_file_bytes.len(),
                    "expected the relocated payload length to match the original payload length of {pe_file_name}"
                );

                let relocated_payload_ptr = byte_array::byte_array_get_raw(ctx, payload_hdl);
                assert!(
                    !relocated_payload_ptr.is_null(),
                    "the relocated payload pointer was null for {pe_file_name}"
                );

                let relocated_payload =
                    std::slice::from_raw_parts(relocated_payload_ptr, relocated_payload_len)
                        .to_vec();

                if test.has_relocations {
                    // Check that we updated the payload with relocation patches
                    assert_ne!(relocated_payload, pe_file_bytes, "expected the relocated payload contents to be different from the original contents of {pe_file_name}");

                    let mut cur = Cursor::new(relocated_payload);
                    cur.set_position(0x12A08);
                    let rva = cur.read_u64::<LittleEndian>().expect(
                        "Could not read the relocated symbol as a 64bit number from {pe_file_name}",
                    );
                    assert_eq!(
                        rva, 0xA328,
                        "unexpected RVA for patched symbol in {pe_file_name}"
                    )
                } else {
                    // Check that the original payload is unchanged because there are no relocations
                    assert_eq!(relocated_payload, pe_file_bytes, "expected the relocated payload contents to be the same because {pe_file_name} has no relocations");
                }

                byte_array::byte_array_raw_free(relocated_payload_ptr, relocated_payload_len);
            };
        }
    }
}
