use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use mmap_rs::{MmapMut, MmapOptions};
use std::cmp::min;

/// GuestMemory is a representation of the guests's
/// physical memory, often referred to as Guest Physical
/// Memory or Guest Physical Addresses (GPA) in Windows
/// Hypervisor Platform
pub struct GuestMemory {
    mem_map: Box<MmapMut>,
}

impl GuestMemory {
    pub fn new(min_size: usize) -> Result<Self> {
        let mem_map = MmapOptions::new(min_size).map_mut()?;
        Ok(Self {
            mem_map: Box::new(mem_map),
        })
    }

    /// Get the base address of guest memory
    ///
    /// # Safety
    ///
    /// This function should not be used to do pointer artithmetic.
    /// Only use it to get the base address of the memory map so you
    /// can do things like calculate offsets, etc...
    pub fn source_address(&self) -> usize {
        let source_address_ptr = self.mem_map.as_ptr();
        source_address_ptr as usize
    }

    /// Copy from_bytes into the guest memory contained within self.
    ///
    /// If from_bytes is smaller than the size of the guest memory within
    /// self, this function does not overwrite the remainder. If it is
    /// larger, this function copy only the first N bytes where N = the
    /// size of guest memory
    pub fn copy_into(&mut self, from_bytes: &[u8]) -> Result<()> {
        let size_to_copy = min(from_bytes.len(), self.mem_map.len());
        self.mem_map
            .as_mut_slice()
            .copy_from_slice(&from_bytes[..size_to_copy]);
        Ok(())
    }

    /// Write val into guest memory at the given offset
    /// from the start of guest memory
    pub fn write_u64(&mut self, offset: usize, val: u64) -> Result<()> {
        // write the u64 into 8 bytes, so we can std::ptr::write
        // them into guest mem
        let mut writer = vec![];
        writer.write_u64::<LittleEndian>(val)?;
        let mut_slice = self.mem_map.as_mut_slice();
        for (idx, item) in writer.iter().enumerate() {
            mut_slice[offset + idx] = *item;
        }
        Ok(())
    }
}
