use crate::mem::layout::SandboxMemoryLayout;

/// Get the VMs base address from the `SandboxMemoryLayout`.
#[no_mangle]
pub extern "C" fn mem_layout_get_base_address() -> usize {
    SandboxMemoryLayout::BASE_ADDRESS
}
