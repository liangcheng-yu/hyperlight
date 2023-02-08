use super::ptr_addr_space::{AddressSpace, GuestAddressSpace, HostAddressSpace};
use super::ptr_offset::Offset;
use super::shared_mem::SharedMemory;
use anyhow::{anyhow, Result};

/// A representation of a raw pointer inside a given address space.
///
/// Use this type to distinguish between an offset and a raw pointer
#[derive(Debug, Clone)]
pub struct RawPtr(pub u64);

impl From<u64> for RawPtr {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

/// Convenience type for representing a pointer into the host address space
pub type HostPtr = Ptr<HostAddressSpace>;
impl TryFrom<(RawPtr, &SharedMemory, bool)> for HostPtr {
    type Error = anyhow::Error;
    /// Create a new `HostPtr` from the given `host_raw_ptr`, which must
    /// be a pointer in the host's address space.
    fn try_from(tup: (RawPtr, &SharedMemory, bool)) -> Result<Self> {
        let (host_raw_ptr, shared_mem, in_mem) = tup;
        HostPtr::from_raw_ptr(HostAddressSpace::new(shared_mem, in_mem), host_raw_ptr)
    }
}
/// Convenience type for representing a pointer into the guest address space
pub type GuestPtr = Ptr<GuestAddressSpace>;
impl TryFrom<(RawPtr, bool)> for GuestPtr {
    type Error = anyhow::Error;
    /// Create a new `GuestPtr` from the given `guest_raw_ptr`, which must
    /// be a pointer in the guest's address space.
    fn try_from(tup: (RawPtr, bool)) -> Result<Self> {
        let (raw, in_mem) = tup;
        GuestPtr::from_raw_ptr(GuestAddressSpace::new(in_mem), raw)
    }
}

/// A pointer into a specific `AddressSpace` `T`.
pub struct Ptr<T: AddressSpace> {
    addr_space: T,
    offset: Offset,
}

impl<T: AddressSpace> Ptr<T> {
    /// Create a new pointer in the given `AddressSpace` `addr_space`
    /// from the given pointer `raw_ptr`. Returns `Ok` if subtracting
    /// the base address from `raw_ptr` succeeds (i.e. does not overflow)
    /// and a `Ptr<T>` can be successfully created
    fn from_raw_ptr(addr_space: T, raw_ptr: RawPtr) -> Result<Ptr<T>> {
        let offset = raw_ptr.0.checked_sub(addr_space.base()).ok_or_else(|| {
            anyhow!(
                "from_raw_ptr: raw pointer ({:?}) was less than the base address ({:?})",
                raw_ptr,
                addr_space.base(),
            )
        })?;
        Ok(Self {
            addr_space,
            offset: Offset::from(offset),
        })
    }

    /// Create a new `Ptr<Tgt>` from a source pointer and a target
    /// address space
    fn from_foreign_ptr<Src: AddressSpace, Tgt: AddressSpace>(
        foreign_ptr: &Ptr<Src>,
        target_addr_space: Tgt,
    ) -> Result<Ptr<Tgt>> {
        let tgt = Ptr {
            addr_space: target_addr_space,
            offset: foreign_ptr.offset(),
        };
        Ok(tgt)
    }

    /// Convert `self` into a new `Ptr` with a different address
    /// space.
    pub fn to_foreign_ptr<Tgt: AddressSpace>(&self, target_addr_space: Tgt) -> Result<Ptr<Tgt>> {
        Self::from_foreign_ptr(self, target_addr_space)
    }

    /// Get the base address for this pointer
    fn base(&self) -> u64 {
        self.addr_space.base()
    }

    /// Get the offset into the pointer's address space
    fn offset(&self) -> Offset {
        self.offset.clone()
    }

    /// Get the absolute value for the pointer represented by `self`.
    ///
    /// This function should rarely be used. Prefer to use offsets
    /// instead.
    pub fn absolute(&self) -> Result<u64> {
        self.base()
            .checked_add(self.offset.as_u64())
            .ok_or_else(|| {
                anyhow!(
                    "couldn't add pointer offset {} to base {}",
                    self.offset.as_u64(),
                    self.base(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::mem::{
        layout::SandboxMemoryLayout,
        ptr_addr_space::{GuestAddressSpace, HostAddressSpace},
        shared_mem::SharedMemory,
    };

    use super::{GuestPtr, HostPtr, RawPtr};
    const OFFSET: u64 = 1;

    #[test]
    fn ptr_basic_ops() {
        {
            let gm = SharedMemory::new(10).unwrap();

            let raw_host_ptr = RawPtr(OFFSET + gm.base_addr() as u64);
            let host_ptr = HostPtr::try_from((raw_host_ptr, &gm, false)).unwrap();
            assert_eq!(OFFSET + gm.base_addr() as u64, host_ptr.absolute().unwrap());
        }
        {
            let raw_guest_ptr = RawPtr(OFFSET + SandboxMemoryLayout::BASE_ADDRESS as u64);
            let guest_ptr = GuestPtr::try_from((raw_guest_ptr, false)).unwrap();
            assert_eq!(
                OFFSET + SandboxMemoryLayout::BASE_ADDRESS as u64,
                guest_ptr.absolute().unwrap()
            );
        }
    }

    #[test]
    fn ptr_fail() {
        // the pointer absolute value is less than the base address of
        // guest memory, so you shouldn't be able to create a host or guest
        // address
        {
            let gm = SharedMemory::new(10).unwrap();

            let raw_host_ptr = RawPtr(gm.base_addr() as u64 - 1);
            let host_ptr = HostPtr::try_from((raw_host_ptr, &gm, false));
            assert!(host_ptr.is_err());
        }
        {
            let raw_guest_ptr = RawPtr(SandboxMemoryLayout::BASE_ADDRESS as u64 - 1);
            let guest_ptr = GuestPtr::try_from((raw_guest_ptr, false));
            assert!(guest_ptr.is_err());
        }
    }

    #[test]
    fn round_trip() {
        let gm = SharedMemory::new(10).unwrap();
        let raw_host_ptr = RawPtr(gm.base_addr() as u64 + OFFSET);

        let host_ptr = {
            let hp = HostPtr::try_from((raw_host_ptr, &gm, false));
            assert!(hp.is_ok());
            let host_ptr = hp.unwrap();
            assert_eq!(OFFSET, host_ptr.offset().as_u64());
            host_ptr
        };

        let guest_ptr = {
            let gp_res = host_ptr.to_foreign_ptr(GuestAddressSpace::new(false));
            assert!(gp_res.is_ok());
            gp_res.unwrap()
        };
        assert_eq!(OFFSET, guest_ptr.offset().as_u64());
        assert_eq!(
            OFFSET + SandboxMemoryLayout::BASE_ADDRESS as u64,
            guest_ptr.absolute().unwrap()
        );

        let ret_host_ptr = {
            let gp = guest_ptr.to_foreign_ptr(HostAddressSpace::new(&gm, false));
            assert!(gp.is_ok());
            gp.unwrap()
        };
        assert_eq!(
            host_ptr.absolute().unwrap(),
            ret_host_ptr.absolute().unwrap()
        );
    }
}
