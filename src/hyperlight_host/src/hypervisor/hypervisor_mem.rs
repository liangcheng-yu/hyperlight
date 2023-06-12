#[cfg(test)]
use crate::mem::shared_mem::SharedMemory;
#[cfg(test)]
use anyhow::Result;

pub(crate) const CR4_PAE: u64 = 1 << 5;
pub(crate) const CR4_OSFXSR: u64 = 1 << 9;
pub(crate) const CR4_OSXMMEXCPT: u64 = 1 << 10;
pub(crate) const CR0_PE: u64 = 1;
pub(crate) const CR0_MP: u64 = 1 << 1;
pub(crate) const CR0_ET: u64 = 1 << 4;
pub(crate) const CR0_NE: u64 = 1 << 5;
pub(crate) const CR0_WP: u64 = 1 << 16;
pub(crate) const CR0_AM: u64 = 1 << 18;
pub(crate) const CR0_PG: u64 = 1 << 31;
pub(crate) const EFER_LME: u64 = 1 << 8;
pub(crate) const EFER_LMA: u64 = 1 << 10;

/// The list of addresses that are required to create a new
/// `HypervLinuxDriver`
#[repr(C)]
#[derive(Default, Debug)]
pub struct HypervisorAddrs {
    /// The location of the first line of code in guest memory
    ///
    /// This generally corresponds to the instruction pointer
    /// (rip).
    pub entrypoint: u64,
    /// The number of page frames that should exist.
    /// One frame = 4k, or 0x1000 bits.
    pub guest_pfn: u64,
    /// The location of the start of memory on the host.
    ///
    /// TODO: instead of this, just put a &SharedMemory in here.
    /// this should be done after the Rust rewrite is complete
    pub host_addr: u64,
    /// Total size of the memory that starts at `host_addr`.
    ///
    /// You must own all bytes in memory in the range
    /// `[*host_addr, *(host_addr + mem_size)]
    ///
    /// TODO: instead of this, just put a &SharedMemory in here.
    /// this should be done after the Rust rewrite is complete
    pub mem_size: u64,
}

impl HypervisorAddrs {
    /// Create a new instance of `HypervisorAddrs`
    /// given a `SharedMemory` and additional metadata.
    ///
    /// The `load_addr` and `mem_size` fields will be set as appropriate
    /// based on the given `shared_mem` parameter.
    #[cfg(test)]
    pub(crate) fn for_shared_mem(
        shared_mem: &SharedMemory,
        region_mem_size: u64,
        entrypoint: u64,
        guest_pfn: u64,
    ) -> Result<Self> {
        Ok(Self {
            entrypoint,
            guest_pfn,
            host_addr: shared_mem.base_addr() as u64,
            mem_size: region_mem_size,
        })
    }
}
