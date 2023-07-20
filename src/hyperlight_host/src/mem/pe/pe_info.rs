use crate::mem::pe::base_relocations;
use anyhow::{anyhow, bail, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use goblin::pe::{optional_header::OptionalHeader, PE};
use std::io::Cursor;
use std::{fs::File, io::Read};
use tracing::info;
use tracing::instrument;

const IMAGE_REL_BASED_DIR64: u8 = 10;
const IMAGE_REL_BASED_ABSOLUTE: u8 = 0;
const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
const CHARACTERISTICS_RELOCS_STRIPPED: u16 = 0x0001;
const CHARACTERISTICS_EXECUTABLE_IMAGE: u16 = 0x0002;

/// An owned representation of a PE file.
///
/// Does not contain comprehensive information about a given
/// PE file, but rather just enough to be able to do relocations,
/// symbol resolution, and actually execute it within a `Sandbox`.
pub(crate) struct PEInfo {
    payload: Vec<u8>,
    payload_len: usize,
    optional_header: OptionalHeader,
}

impl PEInfo {
    #[instrument(err(Debug))]
    pub(crate) fn from_file(filename: &str) -> Result<Self> {
        info!("Loading PE file from {}", filename);
        let mut file = File::open(filename)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        Self::new(contents.as_slice())
    }
    /// Create a new `PEInfo` from a slice of bytes.
    ///
    /// Returns `Ok` with the new `PEInfo` if `pe_bytes` is a valid
    /// PE file and could properly be parsed as such, and `Err` if not.
    pub(crate) fn new(pe_bytes: &[u8]) -> Result<Self> {
        let pe = PE::parse(pe_bytes).map_err(|e| anyhow!(e))?;

        // Validate that the PE file has the expected characteristics up-front
        if pe.header.coff_header.machine != IMAGE_FILE_MACHINE_AMD64 {
            bail!("unsupported PE file, contents is not a x64 File")
        }

        if !pe.is_64 {
            bail!("unsupported PE file, not a PE32+ formatted file")
        }

        if (pe.header.coff_header.characteristics & CHARACTERISTICS_EXECUTABLE_IMAGE)
            != CHARACTERISTICS_EXECUTABLE_IMAGE
        {
            bail!("unsupported PE file, not an executable image")
        }

        let optional_header = pe
            .header
            .optional_header
            .expect("unsupported PE file, missing optional header entry");

        if (pe.header.coff_header.characteristics & CHARACTERISTICS_RELOCS_STRIPPED)
            == CHARACTERISTICS_RELOCS_STRIPPED
        {
            bail!("unsupported PE file, relocations have been removed")
        }

        Ok(Self {
            payload: Vec::from(pe_bytes),
            optional_header,
            payload_len: pe_bytes.len(),
        })
    }

    /// Get a reference to the payload contained within `self`
    pub(crate) fn get_payload(&self) -> &[u8] {
        &self.payload
    }

    /// Get a mutable reference to the payload contained within `self`
    pub(crate) fn get_payload_mut(&mut self) -> &mut [u8] {
        &mut self.payload
    }
    /// Get the length of the entire PE file payload
    pub(crate) fn get_payload_len(&self) -> usize {
        self.payload_len
    }

    /// Get the entry point offset from the PE file's optional COFF
    /// header.
    pub(crate) fn entry_point_offset(&self) -> u64 {
        self.optional_header.standard_fields.address_of_entry_point
    }

    /// Get the load address specified in the PE file's optional COFF header.
    pub(crate) fn preferred_load_address(&self) -> u64 {
        self.optional_header.windows_fields.image_base
    }

    /// Return the stack reserve field from the optional COFF header.
    pub(crate) fn stack_reserve(&self) -> u64 {
        self.optional_header.windows_fields.size_of_stack_reserve
    }

    /// Return the stack commit field from the optional COFF header.
    pub(crate) fn stack_commit(&self) -> u64 {
        self.optional_header.windows_fields.size_of_stack_commit
    }

    /// Return the heap reserve field from the optional COFF header.
    pub(crate) fn heap_reserve(&self) -> u64 {
        self.optional_header.windows_fields.size_of_heap_reserve
    }

    /// Return the heap commit field from the optional COFF header.
    pub(crate) fn heap_commit(&self) -> u64 {
        self.optional_header.windows_fields.size_of_heap_commit
    }

    /// Apply the list of `RelocationPatch`es in `patches` to the given
    /// `payload` and return the number of patches applied.
    pub(crate) fn apply_relocation_patches(
        payload: &mut [u8],
        patches: Vec<RelocationPatch>,
    ) -> Result<usize> {
        let payload_len = payload.len();
        let mut cur = Cursor::new(payload);

        // Track how many patches were applied to the payload
        let mut applied: usize = 0;
        for patch in patches {
            if patch.offset >= payload_len {
                bail!("invalid offset is larger than the payload");
            }

            cur.set_position(patch.offset as u64);
            cur.write_u64::<LittleEndian>(patch.relocated_virtual_address)
                .expect("failed to write patch to pe file contents");
            applied += 1;
        }

        Ok(applied)
    }

    /// Get a list of patches to make to the symbol table to
    /// complete the relocations in the relocation table.
    pub(crate) fn get_exe_relocation_patches(
        &self,
        payload: &[u8],
        address_to_load_at: usize,
    ) -> Result<Vec<RelocationPatch>> {
        // see the following for information on relocations:
        //
        // - https://stackoverflow.com/questions/17436668/how-are-pe-base-relocations-build-up
        // - https://0xrick.github.io/win-internals/pe7/
        // - https://www.codeproject.com/Articles/12532/Inject-your-code-to-a-Portable-Executable-file#ImplementRelocationTable7_2

        // If the exe is loading/loaded at its preferred address there is nothing to do
        let addr_diff = (address_to_load_at as u64).wrapping_sub(self.preferred_load_address());
        if addr_diff == 0 {
            return Ok(Vec::new());
        }

        let relocations = base_relocations::get_base_relocations(payload, self.optional_header)
            .expect("error parsing base relocations");
        let mut patches = Vec::with_capacity(relocations.len());

        for reloc in relocations {
            match reloc.typ {
                // IMAGE_REL_BASED_DIR64:
                // "The base relocation applies the difference to the
                // 64-bit field at offset"
                // see: https://docs.microsoft.com/en-us/windows/win32/debug/pe-format#base-relocation-types
                IMAGE_REL_BASED_DIR64 => {
                    let offset = reloc.page_base_rva as u64 + (reloc.page_offset as u64);

                    // Read the virtual address stored in reloc_offset as a 64bit value
                    let mut cur = Cursor::new(payload);
                    cur.set_position(offset);
                    let original_address = match cur.read_u64::<LittleEndian>() {
                        Ok(val) => val,
                        Err(e) => {
                            bail!("error reading a 64bit value from the PE file at offset {offset}: {e}")
                        }
                    };

                    // Add the address diff to the original address
                    // Note that we are using wrapping when calculating the diff and then again when applying it to the original address
                    // So even though the diff is an unsigned number, we can represent a negative number using 2's complement.
                    // This lets us avoid trying to work with signed and unsigned integers (which isn't supported in stable rust yet).
                    let relocated_virtual_address = original_address.wrapping_add(addr_diff);
                    patches.push(RelocationPatch {
                        offset: offset as usize,
                        relocated_virtual_address,
                    });
                }

                // IMAGE_REL_BASED_ABSOLUTE
                // "The base relocation is skipped. This type can
                // be used to pad a block."
                // see: https://docs.microsoft.com/en-us/windows/win32/debug/pe-format#base-relocation-types
                IMAGE_REL_BASED_ABSOLUTE => (),

                // Give up on any other relocation type
                _ => bail!("unsupported relocation type {}", reloc.typ),
            }
        }
        Ok(patches)
    }
}

/// Represents a patch that relocates a symbol to its final destination.
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub(crate) struct RelocationPatch {
    /// The offset of the address to patch.
    offset: usize,
    /// The new virtual address that should be written at offset.
    relocated_virtual_address: u64,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::fs;

    use crate::testing::{callback_guest_path, simple_guest_path};

    struct PEFileTest {
        path: String,
        stack_size: u64,
        heap_size: u64,
        load_address: u64,
        num_relocations: u8,
    }
    fn pe_files() -> Result<Vec<PEFileTest>> {
        Ok(vec![
            PEFileTest {
                path: simple_guest_path()?,
                stack_size: 65536,
                heap_size: 131072,
                load_address: 5368709120,
                num_relocations: 1,
            },
            PEFileTest {
                path: callback_guest_path()?,
                stack_size: 65536,
                heap_size: 131072,
                load_address: 5368709120,
                num_relocations: 0,
            },
        ])
    }

    #[test]
    fn load_pe_info() -> Result<()> {
        for test in pe_files()? {
            let pe_path = test.path;
            let pe_bytes = fs::read(pe_path.clone())?;
            let pe_info = super::PEInfo::new(&pe_bytes)?;

            // Validate that the pe headers aren't empty
            assert_eq!(
                test.stack_size,
                pe_info.stack_reserve(),
                "unexpected stack reserve for {pe_path}",
            );
            assert_eq!(
                test.stack_size,
                pe_info.stack_commit(),
                "unexpected stack commit for {pe_path}"
            );
            assert_eq!(
                pe_info.heap_reserve(),
                test.heap_size,
                "unexpected heap reserve for {pe_path}",
            );
            assert_eq!(
                pe_info.heap_commit(),
                test.heap_size,
                "unexpected heap commit for {pe_path}",
            );
            assert_eq!(
                pe_info.preferred_load_address(),
                test.load_address,
                "unexpected load address for {pe_path}"
            );

            let patches = pe_info.get_exe_relocation_patches(&pe_bytes, 0).unwrap_or_else(|_| {
                let num_relocations = test.num_relocations;
                panic!("expected {num_relocations} relocation patches to be returned for {pe_path}")
            });
            assert_eq!(
                patches.len(),
                test.num_relocations as usize,
                "unexpected number of relocations for {pe_path}"
            );

            // simple guest is the only test file with relocations, check that it was calculated correctly
            if pe_path.ends_with("simpleguest.exe") {
                let patch = patches[0];
                let expected_patch_offset = if cfg!(debug_assertions) {
                    0x20E98
                } else {
                    0xF098
                };
                assert_eq!(
                    patch.offset, expected_patch_offset,
                    "incorrect patch offset for {pe_path}"
                );
            }
        }
        Ok(())
    }
}
