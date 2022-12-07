use super::config::SandboxMemoryConfiguration;
use super::guest_mem::GuestMemory;
use super::layout::SandboxMemoryLayout;
use crate::mem::pe::PEInfo;
use anyhow::Result;
use goblin::pe::PE;

/// A struct that is responsible for laying out and managing the memory
/// for a given `Sandbox`.
pub struct SandboxMemoryManager {
    run_from_process_memory: bool,
    layout: Box<SandboxMemoryLayout>,
}

impl SandboxMemoryManager {
    /// Create a new `SandboxMemoryManager` for the given guest binary.
    ///
    /// Note: not yet implemented.
    pub fn load_binary_using_load_library(
        _guest_binary_path: String,
        _pe: PE,
        _cfg: SandboxMemoryConfiguration,
    ) -> Result<Self> {
        todo!("see https://github.com/deislabs/hyperlight/blob/0038b1dd16a27113db8f120cca1b090c9bf0c342/src/Hyperlight/Core/SandboxMemoryManager.cs#L34-L62")
    }

    /// Create a new `SandboxMemoryManager` for the given guest binary.
    pub fn load_guest_binary_into_memory(
        pe_payload: &mut [u8],
        pe_info: &PEInfo,
        cfg: SandboxMemoryConfiguration,
        run_from_process_memory: bool,
    ) -> Result<Self> {
        let entry_point_offset = pe_info.try_entry_point_offset()? as usize;

        let layout = Box::new(SandboxMemoryLayout::new(
            cfg,
            pe_payload.len(),
            pe_info.stack_reserve()? as usize,
            pe_info.heap_reserve()? as usize,
        ));
        let mut guest_mem = GuestMemory::new(layout.get_memory_size()?)?;
        let base_address = guest_mem.base_addr();

        let host_code_offset = SandboxMemoryLayout::CODE_OFFSET;
        let host_code_address = base_address + host_code_offset;
        pe_info.relocate_payload(pe_payload, host_code_address)?;

        // If we are running in memory the entry point will
        // be relative to the base_address.
        // If we are running in a Hypervisor it will be
        // relative to 0x230000, which is where the code is
        // loaded in the GPA space.
        if run_from_process_memory {
            let _entry_point = host_code_address + entry_point_offset;
            guest_mem.copy_from_slice(pe_payload, 0)?;
            guest_mem.write_u64(layout.get_code_pointer_offset(), host_code_address as u64)?;
            Ok(Self {
                layout,
                run_from_process_memory,
            })
        } else {
            let _entry_point = SandboxMemoryLayout::GUEST_CODE_ADDRESS + entry_point_offset;
            guest_mem.copy_from_slice(pe_payload, 0)?;
            // TODO:
            // Marshal.Copy(peInfo.HyperVisorPayload, 0, (IntPtr)hostCodeAddress, peInfo.Payload.Length);
            // there is a difference in the C# implementation between
            // HypervisorPayload and Payload. This else branch
            // uses the former, and the above if branch uses the latter.
            // need to replicate that here.

            guest_mem.write_u64(
                layout.get_code_pointer_offset(),
                SandboxMemoryLayout::GUEST_CODE_ADDRESS as u64,
            )?;
            Ok(Self {
                layout,
                run_from_process_memory,
            })
        }
    }

    /// Get the total size of memory needed for this sandbox
    pub fn size(&self) -> Result<usize> {
        self.layout.get_memory_size()
    }

    /// Get the peb address of the `Sandbox` whose memory is managed
    /// by `self`.
    pub fn get_peb_address(&self) -> usize {
        match self.run_from_process_memory {
            true => self.layout.get_in_process_peb_offset(),
            false => self.layout.peb_address,
        }
    }
}
