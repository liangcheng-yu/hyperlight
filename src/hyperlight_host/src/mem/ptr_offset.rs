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
        Self::default()
    }
}

impl Default for Offset {
    fn default() -> Self {
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

impl TryFrom<Offset> for i64 {
    type Error = anyhow::Error;
    fn try_from(val: Offset) -> Result<i64> {
        i64::try_from(val.0).map_err(|_| anyhow!("couldn't convert Offset ({:?}) to i64", val))
    }
}

impl TryFrom<i64> for Offset {
    type Error = anyhow::Error;
    fn try_from(val: i64) -> Result<Offset> {
        let val_u64 = u64::try_from(val)?;
        Ok(Offset::from(val_u64))
    }
}

impl TryFrom<usize> for Offset {
    type Error = anyhow::Error;
    fn try_from(val: usize) -> Result<Offset> {
        u64::try_from(val)
            .map(Offset::from)
            .map_err(|_| anyhow!("couldn't convert usize ({:?}) to Offset", val))
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

#[cfg(test)]
mod tests {
    use super::Offset;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn i64_roundtrip(i64_val in (i64::MIN..i64::MAX)) {
            let offset_res = Offset::try_from(i64_val);

            if i64_val < 0 {
                assert!(offset_res.is_err());
            } else {
                assert!(offset_res.is_ok());
                let offset = offset_res.unwrap();
                let ret_i64_val = {
                    let res = offset.try_into();
                    assert!(res.is_ok());
                    res.unwrap()
                };
                assert_eq!(i64_val, ret_i64_val);
            }
        }
        #[test]
        fn usize_roundtrip(val in (usize::MIN..usize::MAX)) {
            let offset = Offset::try_from(val).unwrap();
            assert_eq!(val, usize::try_from(offset).unwrap());
        }

        #[test]
        fn add_numeric_types(usize_val in (usize::MIN..usize::MAX), u64_val in (u64::MIN..u64::MAX)) {
            let start = Offset::default();
            {
                // add usize to offset
                assert_eq!(usize_val, usize::try_from(start + usize_val).unwrap());
            }
            {
                // add u64 to offset
                assert_eq!(u64_val, u64::from(start + u64_val));
            }
        }
    }
}
