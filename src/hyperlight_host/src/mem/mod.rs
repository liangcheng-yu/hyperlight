///! Configuration needed to establish a sandbox's memory layout.
pub mod config;
///! A wrapper around unsafe functionality to create and initialize
///! a memory region for a guest running in a sandbox.
pub mod guest_mem;
///! A wrapper around a `GuestMemory` and a snapshot in time
///! of the memory therein
pub mod guest_mem_snapshot;
///! Functionality to establish a sandbox's memory layout.
pub mod layout;
///! Functionality taht wraps a `SandboxMemoryLayout` and a
///! `SandboxMemoryConfig` to mutate a sandbox's memory as necessary.
pub mod mgr;
///! Functionality to read and mutate a PE file in a structured manner.
pub mod pe;

use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
/// Write `val` to `slc` as little-endian at `offset.
///
/// If `Ok` is returned, `slc` will have been modified
/// in-place. Otherwise, no modifications will have been
/// made.
pub fn write_u32(slc: &mut [u8], offset: usize, val: u32) -> Result<()> {
    let mut target: Vec<u8> = Vec::new();
    // PE files are always little-endian
    // https://reverseengineering.stackexchange.com/questions/17922/determining-endianness-of-pe-files-windows-on-arm
    target.write_u32::<LittleEndian>(val)?;

    for (idx, elt) in target.iter().enumerate() {
        slc[offset + idx] = *elt;
    }

    Ok(())
}
