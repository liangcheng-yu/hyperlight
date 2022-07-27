//! The following structs are not used other than to calculate the size of the memory needed
//! and also to illustrate the layout of the memory:
//!
//! - `HostFunctionDefinitions`
//! - `HostExceptionData`
//! - `GuestError`
//! - `CodeAndOutBPointers`
//! - `InputData`
//! - `OutputData`
//! - `GuestHeap`
//! - `GuestStack`
//!
//! the start of the guest  memory contains the page tables and is always located at the Virtual Address 0x00200000 when
//! running in a Hypervisor:
//!
//! Virtual Address
//!
//! 0x200000    PML4
//! 0x201000    PDPT
//! 0x202000    PD
//! 0x203000    The guest PE code (When the code has been loaded using LoadLibrary to debug the guest this will not be
//! present and code length will be zero;
//!
//! The pointer passed to the Entrypoint in the Guest application is the 0x200000 + size of page table + size of code,
//! at this address structs below are laid out in this order

pub mod config;
pub mod guest_mem;
pub mod layout;
pub mod mgr;
pub mod pe;

use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};

/// Write `val` to `slc` as little-endian at `offset`.
///
/// If `Ok` is returned, `slc` will have been modified
/// in-place. Otherwise, no modifications will have been
/// made.
fn write_usize(slc: &mut [u8], offset: usize, val: usize) -> Result<()> {
    let mut target: Vec<u8> = Vec::new();
    // PE files are always little-endian
    // https://reverseengineering.stackexchange.com/questions/17922/determining-endianness-of-pe-files-windows-on-arm
    target.write_u64::<LittleEndian>(val as u64)?;
    for (idx, elt) in target.iter().enumerate() {
        slc[offset + idx] = *elt;
    }
    Ok(())
}

/// Write `val` to `slc` as little-endian at `offset.
///
/// If `Ok` is returned, `slc` will have been modified
/// in-place. Otherwise, no modifications will have been
/// made.
fn write_u32(slc: &mut [u8], offset: usize, val: u32) -> Result<()> {
    let mut target: Vec<u8> = Vec::new();
    // PE files are always little-endian
    // https://reverseengineering.stackexchange.com/questions/17922/determining-endianness-of-pe-files-windows-on-arm
    target.write_u32::<LittleEndian>(val)?;

    for (idx, elt) in target.iter().enumerate() {
        slc[offset + idx] = *elt;
    }

    Ok(())
}
