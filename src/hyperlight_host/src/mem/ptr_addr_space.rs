use super::{guest_mem::GuestMemory, layout::SandboxMemoryLayout};

/// A representation of a specific address space
pub trait AddressSpace {
    /// The base address for this address space
    fn base(&self) -> u64;
}

/// The address space for the guest executable
#[derive(Debug)]
pub struct GuestAddressSpace(u64);
impl GuestAddressSpace {
    /// Create a new instance of a `GuestAddressSpace`
    pub fn new(is_in_memory: bool) -> Self {
        let base_addr = if is_in_memory {
            0
        } else {
            SandboxMemoryLayout::BASE_ADDRESS as u64
        };
        Self(base_addr)
    }
}
impl AddressSpace for GuestAddressSpace {
    fn base(&self) -> u64 {
        self.0
    }
}

/// The address space for the host executable
#[derive(Debug)]
pub struct HostAddressSpace(u64);
impl HostAddressSpace {
    /// Create a new instance of a `HostAddressSpace`, using the given
    /// `GuestMemory` as the base address.
    pub fn new(guest_mem: &GuestMemory, is_in_memory: bool) -> Self {
        let base = if is_in_memory {
            0
        } else {
            guest_mem.base_addr() as u64
        };
        Self(base)
    }
}
impl AddressSpace for HostAddressSpace {
    fn base(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::mem::{guest_mem::GuestMemory, layout::SandboxMemoryLayout};

    use super::{AddressSpace, GuestAddressSpace, HostAddressSpace};

    #[test]
    fn host_addr_space_base() {
        let gm = GuestMemory::new(10).unwrap();
        let space = HostAddressSpace::new(&gm, false);
        assert_eq!(gm.base_addr() as u64, space.base());
    }

    #[test]
    fn guest_addr_space_base() {
        let space = GuestAddressSpace::new(false);
        assert_eq!(SandboxMemoryLayout::BASE_ADDRESS as u64, space.base());
    }

    #[test]
    fn in_memory() {
        let gm = GuestMemory::new(1).unwrap();
        let host_addr = HostAddressSpace::new(&gm, true);
        let guest_addr = GuestAddressSpace::new(true);
        assert_eq!(0, host_addr.base());
        assert_eq!(0, guest_addr.base());
    }
}
