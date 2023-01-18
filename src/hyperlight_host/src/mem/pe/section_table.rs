use goblin::pe::section_table::SectionTable;

/// Determine the offset of a value from its relative virtual address (RVA)
/// Looks up the section that contains the RVA, and then applies the difference between the sections's virtual address and the RVA to the section's raw address to determine the offset.
pub fn calculate_offset_from_rva(sections: &[SectionTable], rva: u64) -> Option<u64> {
    for s in sections {
        let section_rva = s.virtual_address as u64;
        if section_rva < rva && section_rva + u64::from(s.virtual_size) > rva {
            let raw_offset = s.pointer_to_raw_data as u64;
            let offset = rva - section_rva + raw_offset;
            return Some(offset);
        }
    }

    None
}
