use tracing::{instrument, Span};

use super::ptr_offset::Offset;
use crate::Result;

/// An extension trait intended for pointer types to which an
/// offset can be added unsafely.
pub(super) trait UnsafeTryAddExt<T> {
    /// The type of the pointer to return
    type PointerType;
    /// The type of the quantity to add to the given pointer.
    type AddType;

    /// Attempt to add an `Offset` to `self`, resulting in a
    /// new pointer
    ///
    /// # Safety
    ///
    /// If `self` is itself a raw pointer, it must point to the
    /// start of an owned memory region that is at least as big
    /// as `to_add`
    unsafe fn try_add(&self, to_add: Self::AddType) -> Result<Self::PointerType>;
}

impl<T> UnsafeTryAddExt<T> for *const T {
    type PointerType = *const T;
    type AddType = Offset;
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    unsafe fn try_add(&self, offset: Offset) -> Result<*const T> {
        let offset_usize = usize::try_from(offset)?;
        let new_ptr = self.add(offset_usize);
        Ok(new_ptr)
    }
}

impl<T> UnsafeTryAddExt<T> for *mut T {
    type PointerType = *mut T;
    type AddType = Offset;
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    unsafe fn try_add(&self, offset: Offset) -> Result<*mut T> {
        let offset_usize = usize::try_from(offset)?;
        let new_ptr = self.add(offset_usize);
        Ok(new_ptr)
    }
}
