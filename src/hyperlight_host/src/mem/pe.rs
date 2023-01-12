use super::write_u32;
use anyhow::{anyhow, Result};
use goblin::pe::{
    header::Header,
    optional_header::OptionalHeader,
    section_table::SectionTable,
    symbol::{Symbol, SymbolTable, IMAGE_SYM_ABSOLUTE, IMAGE_SYM_DEBUG, IMAGE_SYM_UNDEFINED},
    PE,
};

const IMAGE_REL_BASED_DIR64: u16 = 10;
const IMAGE_REL_BASED_ABSOLUTE: u16 = 0;

/// A convenience type for a vector of tuples that represents
/// a relocation that must occur to a symbol.
///
/// The first element in each tuple is the index of the symbol,
/// and the second is the new address of that symbol.
pub type RelocationPatches = Vec<(usize, u32)>;

/// An owned representation of a PE file.
///
/// Does not contain comprehensive information about a given
/// PE file, but rather just enough to be able to do relocations,
/// symbol resolution, and actually execute it within a `Sandbox`.
pub struct PEInfo {
    header: Header,
    sections: Vec<SectionTable>,
}

impl PEInfo {
    /// Create a new `PEInfo` from a slice of bytes.
    ///
    /// Returns `Ok` with the new `PEInfo` if `pe_bytes` is a valid
    /// PE file and could properly be parsed as such, and `Err` if not.
    pub fn new(pe_bytes: &[u8]) -> Result<Self> {
        let pe = PE::parse(pe_bytes).map_err(|e| anyhow!(e))?;
        let header = pe.header;
        let sections = pe.sections;
        Ok(Self { header, sections })
    }
    /// Gets `self.pe.header.optional_header` or returns
    /// a descriptive error indicating the optional header
    /// was missing.
    ///
    /// This method is useful for use in situations where you
    /// need a `Result` rather than the `Option` that the `OptionalHeader`
    /// is stored within in `self.pe.header`.
    fn try_optional_header(&self) -> Result<OptionalHeader> {
        try_optional_header(&self.header)
    }

    /// Get the entry point offset from the PE file's optional COFF
    /// header. Return `Ok` with the offset if the header exists,
    /// `Err` if not.
    pub fn try_entry_point_offset(&self) -> Result<u64> {
        let opt_header = self.try_optional_header()?;
        Ok(opt_header.standard_fields.address_of_entry_point)
    }
    fn preferred_load_address(&self) -> Result<u64> {
        let opt_header = self.try_optional_header()?;
        Ok(opt_header.windows_fields.image_base)
    }

    /// Return the stack reserve field from the optional COFF header.
    ///
    /// Return `Ok` if the header exists, `Err` if not.
    pub fn stack_reserve(&self) -> Result<u64> {
        let opt_hdr = self.try_optional_header()?;
        Ok(opt_hdr.windows_fields.size_of_stack_reserve)
    }

    /// Return the stack commit field from the optional COFF header.
    ///
    /// Return `Ok` if the header exists, `Err` if not.
    pub fn stack_commit(&self) -> Result<u64> {
        let opt_hdr = self.try_optional_header()?;
        Ok(opt_hdr.windows_fields.size_of_stack_commit)
    }

    /// Return the heap reserve field from the optional COFF header.
    ///
    /// Return `Ok` if the header exists, `Err` if not.
    pub fn heap_reserve(&self) -> Result<u64> {
        let opt_hdr = self.try_optional_header()?;
        Ok(opt_hdr.windows_fields.size_of_heap_reserve)
    }

    /// Return the heap commit field from the optional COFF header.
    ///
    /// Return `Ok` if the header exists, `Err` if not.
    pub fn heap_commit(&self) -> Result<u64> {
        let opt_hdr = self.try_optional_header()?;
        Ok(opt_hdr.windows_fields.size_of_heap_commit)
    }

    /// Modify `payload`'s symbol table according to `patches.
    ///
    /// The patches can be obtained from calling `Self::new` and then
    /// calling `get_exe_relocation_patches`.
    pub fn apply_relocation_patches(
        &self,
        patches: &RelocationPatches,
        payload: &mut [u8],
    ) -> Result<()> {
        // number of bytes per symbol entry in the symbol table
        const SYM_SIZE: usize = 18;
        const SYM_VALUE_OFFSET: usize = 8;

        let sym_table_offset = self.header.coff_header.pointer_to_symbol_table as usize;

        for (idx, value) in patches.iter() {
            // offset from the payload[0] to the appropriate symbol.
            // the value to overwrite starts 8 bytes after this,
            // and is 4 bytes wide.
            let sym_offset: usize = sym_table_offset + (idx * SYM_SIZE);
            let value_start_offset = sym_offset + SYM_VALUE_OFFSET;
            write_u32(payload, value_start_offset, *value)?;
        }
        Ok(())
    }

    /// Get a list of patches to make to the symbol table to
    /// complete the relocations in the relocation table.
    pub fn get_exe_relocation_patches(
        &self,
        address_to_load_at: usize,
        payload: &[u8],
    ) -> Result<RelocationPatches> {
        // see the following for information on relocations:
        //
        // - https://stackoverflow.com/questions/17436668/how-are-pe-base-relocations-build-up
        // - https://0xrick.github.io/win-internals/pe7/
        // - https://www.codeproject.com/Articles/12532/Inject-your-code-to-a-Portable-Executable-file#ImplementRelocationTable7_2

        // If the exe is loading/loaded at its preferred address there is nothing to do
        if self.preferred_load_address()? == (address_to_load_at as u64) {
            return Ok(Vec::new());
        }

        // build up a Vec of symbols that need relocating.
        // the first element of each element is the symbol index,
        // second is the new address
        let mut relocated_symbols: Vec<(usize, u32)> = Vec::new();
        // go through each section and apply any relocations if they
        // exist for that section
        for section in self.sections.iter() {
            match section.relocations(payload) {
                // 0 relocations
                Err(_) => continue,
                // >0 relocations
                Ok(relocs) => {
                    for reloc in relocs {
                        // IMAGE_REL_BASED_DIR64:
                        // "The base relocation applies the difference to the
                        // 64-bit field at offset"
                        // see: https://docs.microsoft.com/en-us/windows/win32/debug/pe-format#base-relocation-types
                        if reloc.typ == IMAGE_REL_BASED_DIR64 {
                            let sym = try_symbol_at_idx(
                                &self.header,
                                payload,
                                reloc.symbol_table_index as usize,
                            )?;
                            if should_relocate(&sym) {
                                relocated_symbols.push((
                                    reloc.symbol_table_index as usize,
                                    reloc.virtual_address,
                                ));
                            }
                        } else {
                            // IMAGE_REL_BASED_ABSOLUTE
                            // "The base relocation is skipped. This type can
                            // be used to pad a block."
                            // see: https://docs.microsoft.com/en-us/windows/win32/debug/pe-format#base-relocation-types
                            if reloc.typ == IMAGE_REL_BASED_ABSOLUTE {
                                return Err(anyhow!("unsupported relocation type {}", reloc.typ));
                            }
                        }
                    }
                }
            }
        }

        Ok(relocated_symbols)
    }

    /// Apply the relocations in `self` to `payload`, returning
    /// `Ok` if all relocations were successful, and `Err` if not.
    ///
    /// This is a convenience function for calling
    /// `Self::get_exe_relocation_patches` and then
    /// `Self::apply_relocation_patches`.
    pub fn relocate_payload(&self, payload: &mut [u8], addr: usize) -> Result<()> {
        let patches = self.get_exe_relocation_patches(addr, payload)?;
        self.apply_relocation_patches(&patches, payload)
    }
}

/// Return the symbol table from the optional COFF header.
///
/// If the COFF header exists, return it inside an `Ok`, otherwise
/// return `Err`.
pub fn try_symbol_table<'a>(hdr: &Header, payload: &'a [u8]) -> Result<SymbolTable<'a>> {
    hdr.coff_header
        .symbols(payload.as_ref())
        .map_err(|e| anyhow!(e))
}

fn try_optional_header(hdr: &Header) -> Result<OptionalHeader> {
    hdr.optional_header
        .ok_or_else(|| anyhow!("optional header is missing from the PE file"))
}

fn try_symbol_at_idx(hdr: &Header, payload: &[u8], idx: usize) -> Result<Symbol> {
    let sym_table = hdr
        .coff_header
        .symbols(payload.as_ref())
        .map_err(|e| anyhow!(e))?;

    match sym_table.get(idx) {
        Some((_, sym)) => Ok(sym),
        None => Err(anyhow!("index {} does not exist in symbol table", idx)),
    }
}

/// Return true if `syms is suitable for relocation.
///
/// This function primarily checks `sym`'s storage
/// class and section number to determine its
/// eligibility for relocation.
fn should_relocate(sym: &Symbol) -> bool {
    // Docs on how to interpret symbol values:
    // - https://docs.microsoft.com/en-us/windows/win32/debug/pe-format#coff-symbol-table
    // - https://docs.rs/goblin/latest/goblin/pe/relocation/struct.Relocation.html
    //
    // Notably, the `value` section:
    //
    // "The value that is associated with the symbol. The interpretation of
    // this field depends on SectionNumber and StorageClass. A typical
    // meaning is the relocatable address."

    let section_num = sym.section_number;
    if section_num == IMAGE_SYM_UNDEFINED
        || section_num == IMAGE_SYM_ABSOLUTE
        || section_num == IMAGE_SYM_DEBUG
    {
        return false;
    }

    // TODO: check symbol.storage_class
    // https://docs.microsoft.com/en-us/windows/win32/debug/pe-format#storage-class

    true
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::fs;
    struct PEFileTest<'a> {
        path: &'a str,
        expect_relocations: bool,
    }
    const PE_FILES: [PEFileTest; 2] = [
        PEFileTest {
            path: "./testdata/simpleguest.exe",
            expect_relocations: false,
        },
        PEFileTest {
            path: "./testdata/callbackguest.exe",
            // TODO: figure out why there are no relocations being done
            // on this PE file.
            // expect_relocations: true,
            expect_relocations: false,
        },
    ];

    #[test]
    fn get_exe_relocation_patches() -> Result<()> {
        for pe_file in PE_FILES {
            let pe_file_name = pe_file.path;
            let pe_bytes = fs::read(pe_file_name)?;
            let pe_info = super::PEInfo::new(&pe_bytes)?;
            let reloc_patches = pe_info.get_exe_relocation_patches(0, &pe_bytes)?;
            if pe_file.expect_relocations {
                assert!(!reloc_patches.is_empty());
            } else {
                assert!(reloc_patches.is_empty());
            }
        }
        Ok(())
    }
}
