use anyhow::Result;

use crate::mem::shared_mem::SharedMemory;

/// The list of addresses that are required to create a new
/// `HypervLinuxDriver`
#[repr(C)]
#[derive(Default, Debug)]
pub struct HypervLinuxDriverAddrs {
    /// The location of the first line of code in guest memory
    ///
    /// This generally corresponds to the instruction pointer
    /// (rip).
    pub entrypoint: u64,
    /// The number of page frames that should exist.
    /// One frame = 4k, or 0x1000 bits.
    pub guest_pfn: u64,
    /// The location of the start of memory on the host.
    pub host_addr: u64,
    /// Total size of the memory that starts at `host_addr`.
    ///
    /// You must own all bytes in memory in the range
    /// `[*host_addr, *(host_addr + mem_size)]
    pub mem_size: u64,
}

impl HypervLinuxDriverAddrs {
    /// Create a new instance of `HypervLinuxDriverAddrs`
    /// given a `SharedMemory` and additional metadata.
    ///
    /// The `load_addr` and `mem_size` fields will be set as appropriate
    /// based on the given `shared_mem` parameter.
    pub fn for_shared_mem(
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
