use anyhow::{anyhow, Result};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::convert::From;
use std::ops::Add;

/// An offset into a given address space.
///
/// Use this type to distinguish between an offset and a raw pointer
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Offset(u64);

impl Offset {
    /// Get the offset representing `0`
    pub fn zero() -> Self {
        Offset::from(0_u64)
    }
}
impl From<u64> for Offset {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl From<&Offset> for u64 {
    fn from(val: &Offset) -> u64 {
        val.0
    }
}

impl From<Offset> for u64 {
    fn from(val: Offset) -> u64 {
        val.0
    }
}

impl TryFrom<usize> for Offset {
    type Error = anyhow::Error;
    fn try_from(val: usize) -> Result<Offset> {
        let val_u64 = u64::try_from(val)?;
        Ok(Offset::from(val_u64))
    }
}

/// Convert an `Offset` to a `usize`, returning an `Err` if the
/// conversion couldn't be made.
impl TryFrom<&Offset> for usize {
    type Error = anyhow::Error;
    fn try_from(val: &Offset) -> Result<usize> {
        usize::try_from(val.0).map_err(|e| anyhow!("converting Offset to usize: {}", e))
    }
}

impl TryFrom<Offset> for usize {
    type Error = anyhow::Error;
    fn try_from(val: Offset) -> Result<usize> {
        usize::try_from(&val)
    }
}

impl Add<Offset> for Offset {
    type Output = Offset;

    fn add(self, rhs: Offset) -> Offset {
        Offset::from(self.0 + rhs.0)
    }
}

impl Add<usize> for Offset {
    type Output = Offset;
    fn add(self, rhs: usize) -> Offset {
        Offset(self.0 + rhs as u64)
    }
}

impl Add<Offset> for usize {
    type Output = Offset;
    fn add(self, rhs: Offset) -> Offset {
        rhs.add(self)
    }
}

impl Add<u64> for Offset {
    type Output = Offset;
    fn add(self, rhs: u64) -> Offset {
        Offset(self.0 + rhs)
    }
}

impl Add<Offset> for u64 {
    type Output = Offset;
    fn add(self, rhs: Offset) -> Offset {
        rhs.add(self)
    }
}

impl PartialEq<usize> for Offset {
    fn eq(&self, other: &usize) -> bool {
        if let Ok(offset_usize) = usize::try_from(self) {
            offset_usize == *other
        } else {
            false
        }
    }
}

impl PartialOrd<usize> for Offset {
    fn partial_cmp(&self, rhs: &usize) -> Option<Ordering> {
        match usize::try_from(self) {
            Ok(offset_usize) if offset_usize > *rhs => Some(Ordering::Greater),
            Ok(offset_usize) if offset_usize == *rhs => Some(Ordering::Equal),
            Ok(_) => Some(Ordering::Less),
            Err(_) => None,
        }
    }
}

impl PartialEq<u64> for Offset {
    fn eq(&self, rhs: &u64) -> bool {
        u64::from(self) == *rhs
    }
}

impl PartialOrd<u64> for Offset {
    fn partial_cmp(&self, rhs: &u64) -> Option<Ordering> {
        let lhs: u64 = self.into();
        match lhs > *rhs {
            true => Some(Ordering::Greater),
            false if lhs == *rhs => Some(Ordering::Equal),
            false => Some(Ordering::Less),
        }
    }
}
