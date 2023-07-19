use crate::mem::layout::SandboxMemoryLayout;

/// Get the VMs base address from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_base_address() -> usize {
    SandboxMemoryLayout::BASE_ADDRESS
}

/// Get the pml4 offset from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_pml4_offset() -> usize {
    SandboxMemoryLayout::PML4_OFFSET
}

/// Get the pd offset from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_pd_offset() -> usize {
    SandboxMemoryLayout::PD_OFFSET
}

/// Get the pdpt offset from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_pdpt_offset() -> usize {
    SandboxMemoryLayout::PDPT_OFFSET
}

/// Get the size of the page tables from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_page_table_size() -> usize {
    SandboxMemoryLayout::PAGE_TABLE_SIZE
}
