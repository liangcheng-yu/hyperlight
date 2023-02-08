///! Configuration needed to establish a sandbox's memory layout.
pub mod config;
///! Functionality to establish a sandbox's memory layout.
pub mod layout;
///! Functionality taht wraps a `SandboxMemoryLayout` and a
///! `SandboxMemoryConfig` to mutate a sandbox's memory as necessary.
pub mod mgr;
///! Functionality to read and mutate a PE file in a structured manner.
pub mod pe;
///! Structures to represent pointers into guest and host memory
pub mod ptr;
///! Structures to represent memory address spaces into which pointers
///! point.
pub mod ptr_addr_space;
///! Structures to represent an offset into a memory space
pub mod ptr_offset;
///! A wrapper around unsafe functionality to create and initialize
///! a memory region for a guest running in a sandbox.
pub mod shared_mem;
///! A wrapper around a `SharedMemory` and a snapshot in time
///! of the memory therein
pub mod shared_mem_snapshot;

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
