use super::ptr_addr_space::{AddressSpace, GuestAddressSpace, HostAddressSpace};
use super::ptr_offset::Offset;
use super::shared_mem::SharedMemory;
use anyhow::{anyhow, Result};
use std::ops::Add;

/// A representation of a raw pointer inside a given address space.
///
/// Use this type to distinguish between an offset and a raw pointer
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RawPtr(u64);

impl From<u64> for RawPtr {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl Add<Offset> for RawPtr {
    type Output = RawPtr;
    fn add(self, rhs: Offset) -> RawPtr {
        let val = self.0 + u64::from(rhs);
        RawPtr(val)
    }
}

impl TryFrom<usize> for RawPtr {
    type Error = anyhow::Error;
    fn try_from(val: usize) -> Result<Self> {
        let val_u64 = u64::try_from(val)?;
        Ok(Self::from(val_u64))
    }
}

impl TryFrom<RawPtr> for usize {
    type Error = anyhow::Error;
    fn try_from(val: RawPtr) -> Result<usize> {
        usize::try_from(val.0).map_err(|_| {
            anyhow!(
                "try_from converting RawPtr -> usize: could not convert raw pointer val {:?} to usize",
                val.0
            )
        })
    }
}

impl From<RawPtr> for u64 {
    fn from(val: RawPtr) -> u64 {
        val.0
    }
}

impl From<&RawPtr> for u64 {
    fn from(val: &RawPtr) -> u64 {
        val.0
    }
}

/// Convenience type for representing a pointer into the host address space
pub type HostPtr = Ptr<HostAddressSpace>;
impl TryFrom<(RawPtr, &SharedMemory)> for HostPtr {
    type Error = anyhow::Error;
    /// Create a new `HostPtr` from the given `host_raw_ptr`, which must
    /// be a pointer in the host's address space.
    fn try_from(tup: (RawPtr, &SharedMemory)) -> Result<Self> {
        let (host_raw_ptr, shared_mem) = tup;
        let addr_space = HostAddressSpace::new(shared_mem)?;
        HostPtr::from_raw_ptr(addr_space, host_raw_ptr)
    }
}

impl TryFrom<(Offset, &SharedMemory)> for HostPtr {
    type Error = anyhow::Error;

    fn try_from(tup: (Offset, &SharedMemory)) -> Result<Self> {
        Ok(Self {
            addr_space: HostAddressSpace::new(tup.1)?,
            offset: tup.0,
        })
    }
}
/// Convenience type for representing a pointer into the guest address space
pub type GuestPtr = Ptr<GuestAddressSpace>;
impl TryFrom<RawPtr> for GuestPtr {
    type Error = anyhow::Error;
    /// Create a new `GuestPtr` from the given `guest_raw_ptr`, which must
    /// be a pointer in the guest's address space.
    fn try_from(raw: RawPtr) -> Result<Self> {
        GuestPtr::from_raw_ptr(GuestAddressSpace::new()?, raw)
    }
}

impl TryFrom<Offset> for GuestPtr {
    type Error = anyhow::Error;
    fn try_from(val: Offset) -> Result<Self> {
        let addr_space = GuestAddressSpace::new()?;
        Ok(Ptr::from_offset(addr_space, val))
    }
}

/// A pointer into a specific `AddressSpace` `T`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

    /// Create a new `Ptr` into the given `addr_space` from the given
    /// `offset`.
    pub(crate) fn from_offset(addr_space: T, offset: Offset) -> Ptr<T> {
        Self { addr_space, offset }
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
    pub fn offset(&self) -> Offset {
        self.offset
    }

    /// Get the absolute value for the pointer represented by `self`.
    ///
    /// This function should rarely be used. Prefer to use offsets
    /// instead.
    pub fn absolute(&self) -> Result<u64> {
        let offset_u64: u64 = self.offset.into();
        self.base().checked_add(offset_u64).ok_or_else(|| {
            anyhow!(
                "couldn't add pointer offset {} to base {}",
                offset_u64,
                self.base(),
            )
        })
    }
}

impl<T: AddressSpace> Add<Offset> for Ptr<T> {
    type Output = Ptr<T>;
    fn add(self, rhs: Offset) -> Self::Output {
        Self {
            addr_space: self.addr_space,
            offset: self.offset + rhs,
        }
    }
}

impl<T: AddressSpace> TryFrom<Ptr<T>> for usize {
    type Error = anyhow::Error;
    fn try_from(val: Ptr<T>) -> Result<usize> {
        let abs = val.absolute()?;
        usize::try_from(abs).map_err(|_| {
            anyhow!(
                "try_from Ptr -> usize: could not convert absolute address {:?} to usize",
                abs
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
            let host_ptr = HostPtr::try_from((raw_host_ptr, &gm)).unwrap();
            assert_eq!(OFFSET + gm.base_addr() as u64, host_ptr.absolute().unwrap());
        }
        {
            let raw_guest_ptr = RawPtr(OFFSET + SandboxMemoryLayout::BASE_ADDRESS as u64);
            let guest_ptr = GuestPtr::try_from(raw_guest_ptr).unwrap();
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
            let host_ptr = HostPtr::try_from((raw_host_ptr, &gm));
            assert!(host_ptr.is_err());
        }
        {
            let raw_guest_ptr = RawPtr(SandboxMemoryLayout::BASE_ADDRESS as u64 - 1);
            let guest_ptr = GuestPtr::try_from(raw_guest_ptr);
            assert!(guest_ptr.is_err());
        }
    }

    #[test]
    fn round_trip() {
        let gm = SharedMemory::new(10).unwrap();
        let raw_host_ptr = RawPtr(gm.base_addr() as u64 + OFFSET);

        let host_ptr = {
            let hp = HostPtr::try_from((raw_host_ptr, &gm));
            assert!(hp.is_ok());
            let host_ptr = hp.unwrap();
            assert_eq!(OFFSET, host_ptr.offset().into());
            host_ptr
        };

        let guest_ptr = {
            let gp_res = host_ptr.to_foreign_ptr(GuestAddressSpace::new().unwrap());
            assert!(gp_res.is_ok());
            gp_res.unwrap()
        };
        assert_eq!(OFFSET, guest_ptr.offset().into());
        assert_eq!(
            OFFSET + SandboxMemoryLayout::BASE_ADDRESS as u64,
            guest_ptr.absolute().unwrap()
        );

        let ret_host_ptr = {
            let gp = guest_ptr.to_foreign_ptr(HostAddressSpace::new(&gm).unwrap());
            assert!(gp.is_ok());
            gp.unwrap()
        };
        assert_eq!(
            host_ptr.absolute().unwrap(),
            ret_host_ptr.absolute().unwrap()
        );
    }
}

#[cfg(test)]
mod prop_tests {
    use super::{HostPtr, RawPtr};
    use crate::mem::ptr_addr_space::{GuestAddressSpace, HostAddressSpace};
    use crate::mem::{layout::SandboxMemoryLayout, shared_mem::SharedMemory};
    use proptest::prelude::*;
    proptest! {
        #[test]
        fn test_round_trip(
            offset in 1_u64..1000_u64,
            guest_mem_size in 10_usize..100_usize,
        ) {
            let shared_mem = SharedMemory::new(guest_mem_size).unwrap();
            let raw_host_ptr = RawPtr(shared_mem.base_addr() as u64 + offset);
            let host_ptr = {
                let hp = HostPtr::try_from((raw_host_ptr, &shared_mem));
                assert!(hp.is_ok());
                let host_ptr = hp.unwrap();
                assert_eq!(offset, host_ptr.offset().into());
                host_ptr
            };

            let guest_ptr = {
                let gp_res = host_ptr.to_foreign_ptr(GuestAddressSpace::new().unwrap());
                assert!(gp_res.is_ok());
                gp_res.unwrap()
            };

            assert_eq!(offset, guest_ptr.offset().into());
            assert_eq!(
            offset + SandboxMemoryLayout::BASE_ADDRESS as u64,
                guest_ptr.absolute().unwrap()
            );

        let ret_host_ptr = {
            let gp = guest_ptr.to_foreign_ptr(HostAddressSpace::new(&shared_mem).unwrap());
            assert!(gp.is_ok());
            gp.unwrap()
        };
        assert_eq!(
            host_ptr.absolute().unwrap(),
            ret_host_ptr.absolute().unwrap()
        );

        }
    }
}
