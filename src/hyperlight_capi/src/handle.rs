use crate::validate_context_or_panic;

use super::context::Context;
use super::hdl::Hdl;
use rand::random;

/// An opaque reference to memory within a `Context`.
///
/// Most of the Hyperlight C API uses `Handle`s to refer to
/// various different types. Generally speaking, a user should
/// create a `Handle` to represent some type, and then call
/// `handle_free` exactly once when they're done with it.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Handle(pub u64);

/// The ID of the type of the `Handle`, often used inside
/// C functions to identify the type of a given `Handle`.
pub(crate) type TypeID = u32;
/// The key used to store a `Handle` inside a `Context`.
pub(crate) type Key = u32;

/// The `Key` specifically intended to identify an "empty"
/// handle.
///
/// Use this key to indicate that a `Handle` points to no
/// memory.
pub(crate) const EMPTY_KEY: Key = 0;

/// The `Key` specifically intended to identify an "invalid"
/// handle.
///
/// Use this key to indicate that a `Handle` points to invalid
/// memory.
pub(crate) const INVALID_KEY: Key = 1;

/// The `Key` specifically intended to identify an "null context"
/// handle.
///
/// Use this key to indicate that a `Handle` points to an error
/// indicating a null context was passed to the c api.
pub(crate) const NULL_CONTEXT_KEY: Key = 2;

/// Create and return a new `Key` from a random number.
pub(crate) fn new_key() -> Key {
    let r = random();
    if r == EMPTY_KEY {
        r + 1
    } else {
        r
    }
}

impl From<Hdl> for Handle {
    fn from(hdl: Hdl) -> Self {
        let type_id_shifted = (hdl.type_id() as u64) << 32;
        let key_u64 = hdl.key() as u64;
        Handle(type_id_shifted + key_u64)
    }
}

impl Handle {
    /// Create a new empty `Handle`.
    ///
    /// Empty `Handle`s have no key and are not saved
    /// in any `Context.
    pub(crate) fn new_empty() -> Handle {
        Handle::from(Hdl::Empty())
    }

    /// Return a new `Handle` that is invalid and guaranteed
    /// not to reference any memory inside any `Context`.
    pub(crate) fn new_invalid() -> Handle {
        Handle::from(Hdl::Invalid())
    }

    /// Return a new NullContext error `Handle`.
    pub(crate) fn new_null_context() -> Handle {
        Handle::from(Hdl::NullContext())
    }

    /// Get the key portion of `self`.
    pub(crate) fn key(&self) -> Key {
        // MASK is 32 zero bits followed by 32 one bits:
        //
        // {0 ...}        {1 ...}
        //  ^ 32 of these  ^ 32 of these
        const MASK: u64 = u32::MAX as u64;
        // the left 32 bits of self are the type ID and the
        // right 32 bits are the key:
        //
        // { 32 bits for the type ID }{ 32 bits for the key }
        //
        // we want to turn the left 32 bits -- the type ID -- to
        // zero. we do this with a bitwise AND with MASKL.
        // the result should be:
        //
        // { 0 ... }{ 32 bits for the key }
        //  ^ 32 of these
        let masked = self.0 & MASK;
        // now that we have the leftmost 32 bits as zero, we simply
        // want to truncate them. when we cast our u64 to a u32,
        // we lose the leftmost 32 bits, effectively truncating.
        masked as u32
    }

    /// Get the `TypeID` of `self`.
    ///
    /// This is often useful for comparing to `TypeID` passed in from
    /// C code.
    pub(crate) fn type_id(&self) -> TypeID {
        // MASK is 32 one bits followed by 32 zero bits:
        //
        // { 1 ... }      { 0 ... }
        // ^ 32 of these  ^ 32 of these
        const MASK: u64 = (u32::MAX as u64) << 32;
        // the left 32 bits of self are the type ID, and right
        // 32 bits are the key
        //
        // { 32 bits for the type ID }{ 32 bits for the key }
        //
        // we first want to turn them to zero by doing bitwise
        // AND with MASK. The result will be:
        //
        // { 32 bits for the type ID } { 0 ... }
        //                              ^ 32 of these
        let masked = self.0 & MASK;
        // next, shift the left 32 bits to the right by 32 bits.
        // the result will be:
        //
        // {0 ...} { 32 bits for the type ID}
        //  ^ 32 of these
        //
        // Now we have the 32 bits for the type ID on the
        // right side, where we want them.
        let shifted = masked >> 32;
        // the final step is to truncate the left 32 bits, which
        // are all 0's. the result is:
        //
        // {32 bits for the type ID}
        //
        // we do this by simply casting the previously bitshifted
        // result to a u32.
        shifted as u32
    }
}

/// Return a new `Handle` that is empty.
///
/// This function is unlikely to be useful in production code and is provided
/// for debug purposes.
#[no_mangle]
pub extern "C" fn handle_new_empty() -> Handle {
    Handle::new_empty()
}

/// Free the memory associated with `hdl`.
///
/// # Safety
///
/// You must only call this function exactly once per `Handle`, and only
/// call it after you're done using `hdl`.
///
/// Additionally, `ctx` must be a valid `Context` created with `context_new`
/// and owned by the caller. It must not be modified or deleted while this
/// function is executing.
#[no_mangle]
pub unsafe extern "C" fn handle_free(ctx: *mut Context, hdl: Handle) -> bool {
    validate_context_or_panic!(ctx);

    (*ctx).remove(hdl, |_| true)
}

/// Return `true` if `handle1 == handle2`, `false` otherwise.
#[no_mangle]
pub extern "C" fn handles_equal(handle1: Handle, handle2: Handle) -> bool {
    handle1.0 == handle2.0
}

#[cfg(test)]
mod tests {
    use super::super::hdl::Hdl;
    use super::Handle;
    use super::{new_key, Key};
    use anyhow::Result;

    #[test]
    fn new_empty() -> Result<()> {
        let inv = Handle::new_empty();
        let hdl = Hdl::try_from(inv).expect("expected new_empty to return a handle");
        assert_eq!(crate::handle::EMPTY_KEY, hdl.key());
        Ok(())
    }

    #[test]
    fn new_invalid() -> Result<()> {
        let inv = Handle::new_invalid();
        let hdl = Hdl::try_from(inv).expect("expected new_invalid to return a handle");
        assert_eq!(crate::handle::INVALID_KEY, hdl.key());
        Ok(())
    }

    #[test]
    fn new_null_context() -> Result<()> {
        let inv = Handle::new_null_context();
        let hdl = Hdl::try_from(inv).expect("expected new_null_context to return a handle");
        assert_eq!(crate::handle::NULL_CONTEXT_KEY, hdl.key());
        Ok(())
    }

    #[test]
    fn handle_key() -> Result<()> {
        let mut keys: Vec<Key> = vec![];
        let mut handles: Vec<Handle> = vec![];
        for _ in 0..100 {
            let key = new_key();
            let handle = Handle::from(Hdl::Sandbox(key));
            keys.push(key);
            handles.push(handle);
        }

        for (idx, key) in keys.iter().enumerate() {
            let handle = handles[idx];
            assert_eq!(handle.key(), *key);
        }
        Ok(())
    }

    #[test]
    fn round_trip() -> Result<()> {
        for _ in 0..100 {
            let key = new_key();
            let hdl = Hdl::Sandbox(key);
            let hdl_copy = hdl.clone();
            let handle = Handle::from(hdl);
            let hdl2 = Hdl::try_from(handle)?;
            assert_eq!(hdl_copy, hdl2);
        }

        Ok(())
    }
}
