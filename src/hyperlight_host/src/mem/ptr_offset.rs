use std::convert::From;

/// An offset into a given address space.
///
/// Use this type to distinguish between an offset and a raw pointer
#[derive(Debug, Clone)]
pub struct Offset(u64);

impl Offset {
    /// Convert an `Offset` to a `u64`.
    ///
    /// This is provided instead of an `impl From<u64> for Offset` because
    /// it merely borrows `self` rather than the `From` implementation,
    /// which consumes it.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}
impl From<u64> for Offset {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl From<usize> for Offset {
    fn from(val: usize) -> Self {
        Self::from(val as u64)
    }
}
