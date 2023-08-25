use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

/// A container for a `Vec<T>` that can convert it to/from a raw
/// pointer, suitable for C-compatible APIs.
#[derive(PartialEq, Eq, Debug)]
pub(crate) struct RawVec<T: Copy> {
    internal: Vec<T>,
}

impl<T: Copy> RawVec<T> {
    /// Take ownership over the memory in the range `[ptr, ptr+len)`
    /// (Note the range is exclusive of `ptr+len`), then put it into
    /// a new `RawVec` and return the result.
    ///
    /// # Safety
    ///
    /// `ptr` must point to the beginning of a contiguous, owned memory region
    /// of size `len * sizeof(T)`. After this function returns, the `RawVec`
    /// owns that memory, so you must not do any of the following:
    ///
    /// - Free it manually
    /// - Interact with it in any way except directly or transitively via
    /// the `RawPtr` API
    /// - Pass the same pointer to this function more than once
    pub(crate) unsafe fn from_ptr(ptr: *mut T, len: usize) -> Self {
        let vec = Vec::from_raw_parts(ptr, len, len);
        Self { internal: vec }
    }

    /// Copy the memory in the range `[ ptr, ptr + len*sizeof(T) )`
    /// (note the right side of that range is exclusive)
    /// and store it in a new instance of `RawVec`.
    ///
    /// # Safety
    ///
    /// Since this function copies memory, the caller is still
    /// responsible for the memory in the aforementioned range.
    pub(crate) unsafe fn copy_from_ptr(ptr: *mut T, len: usize) -> Self {
        // convert the (ptr, len) to a native Rust Vec, then immediately
        // move it into a ManuallyDrop so we can avoid dropping it for the
        // moment.
        let nodrop_vec = ManuallyDrop::new(Vec::from_raw_parts(ptr, len, len));
        Self {
            // next, clone the newly-created vector -- to actually copy
            // the memory to which ptr points -- and then move it
            // back to be managed by Rust using ManuallyDrop::into_inner
            internal: ManuallyDrop::into_inner(nodrop_vec.clone()),
        }
    }

    /// Copy the contents of the contained `Vec<T>` into new memory,
    /// then return a pointer to the start of that memory and its length.
    ///
    /// # Safety
    ///
    /// This function is not unsafe because of Rust's soundness guarantees.
    /// In short, just passing a pointer around is safe. Interacting with
    /// the memory behind that pointer -- dereferencing it, for example --
    /// is unsafe.
    ///
    /// Additionally, the pointer that's returned points to memory
    /// for which the caller is responsible. Specifically, the pointer
    /// will point to the start of a memory region of size `len * sizeof(T)`,
    /// where `len` is the value of the second return value.
    ///
    /// You must clean up this memory _only_ by passing the returned pointer
    /// and length to `RawVec::from_ptr(ptr, len)`.
    ///
    /// If you don't do exactly that, or if you pass the pointer to other
    /// functions like `free()`, the results will be undefined, but you'll
    /// at least leak memory.
    #[cfg(test)]
    pub(crate) fn copy_to_ptr(&self) -> (*mut T, usize) {
        // the memory operations are explicitly documented
        // stepwise, below. This function could be compressed down to
        // two lines, but it is purposely left expanded.

        // creates a new, owned Vec inside this function
        let cloned = self.internal.clone();
        // moves cloned into the method, then ensures cloned has
        // no excess capacity (e.g. cloned.len() == cloned.capacity()),
        // and finally returns the slice in a Box (on the heap)
        //
        // according to its documentation, this function may
        // do a copy of cloned, but that's not guaranteed. the
        // lack of guarantee is why we need to do a clone above.
        let slc_box = cloned.into_boxed_slice();
        // ensures that slc_box is not automatically dropped when
        // this function returns, so we can return the pointer to
        // the start of the box and its length to the caller
        // of this function.
        //
        // note that the 'cloned' variable was dropped in the
        // into_boxed_slice method, but its backing memory on the
        // heap was not.
        let mut slc = ManuallyDrop::new(slc_box);
        (slc.as_mut_ptr(), slc.len())
    }
}

impl<T: Copy> From<RawVec<T>> for (*mut T, usize) {
    /// Consume the `RawVec` `value` and return the underlying `*mut T`
    /// pointer and the length of owned memory to which that pointer
    /// points.
    ///
    /// No memory is copied here. If you wish to lift this pointer/size
    /// tuple back into a `RawVec` use the (unsafe) `RawVec::from_ptr`
    /// function.
    fn from(value: RawVec<T>) -> Self {
        // since self.internal is a Vec, we have to deal with the following
        // values before we return:
        //
        // 1. the capacity of the `Vec` (`capacity >= len` at all times)
        // 2. the length of the `Vec`
        // 3. the underlying memory, which always length:
        // `capacity * size_of::<T>()` (not length!)
        //
        // we're returning only a length, not a capacity, so we want to make
        // sure `capacity = length` before we forget about the actual `Vec`
        // and return its underlying pointer. failure to do so will
        // result in a leak of `(capacity - len) * size_of::<T>()` memory,
        // since our `from_ptr` method passes the `length` to both the
        // length and capacity parameters of `Vec::from_raw_parts`.
        //
        // `into_boxed_slice` is the only method on `Vec` I could find that
        // effectively makes `capacity = len`. importantly, `shrink_to_fit`
        // doesn't _guarantee_ that behavior despite its name.
        //
        // Finally, we aren't (and don't want to have to) returning both
        // a length and a capacity, so we want to make sure `capacity = length`
        // before we forget about the `Vec`, so we don't potentially
        // leak `(capacity - length) * size_of::<T>()` memory.
        //
        // See https://stackoverflow.com/a/39693977/78455 for a little
        // more detail.
        let mut slc = ManuallyDrop::new(value.internal.into_boxed_slice());
        (slc.as_mut_ptr(), slc.len())
    }
}

impl<T: Copy> Deref for RawVec<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<T: Copy> DerefMut for RawVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.internal
    }
}

impl<T: Copy> Clone for RawVec<T> {
    fn clone(&self) -> Self {
        Self {
            internal: self.internal.clone(),
        }
    }
}

impl<T: Copy> From<Vec<T>> for RawVec<T> {
    fn from(val: Vec<T>) -> Self {
        Self { internal: val }
    }
}

impl<T: Copy> From<RawVec<T>> for Vec<T> {
    fn from(val: RawVec<T>) -> Vec<T> {
        val.internal
    }
}

#[cfg(test)]
mod tests {

    use super::RawVec;

    #[test]
    fn basic_round_trip_to_ptr() {
        let orig_vec = vec![1, 2, 3, 4, 5, 6];
        let start = RawVec::from(orig_vec);
        let (ptr, len): (*mut i32, usize) = start.clone().into();
        let ret = unsafe { RawVec::from_ptr(ptr, len) };
        assert_eq!(start, ret);
    }

    #[test]
    fn basic_round_trip_copy_to_ptr() {
        let orig_vec: Vec<u64> = vec![1, 2, 3, 4, 5, 6];
        let start = RawVec::from(orig_vec);
        // copy the starting RawVec to 100 different pointers
        let ptrs: Vec<(*mut u64, usize)> = (0..100)
            .collect::<Vec<u64>>()
            .iter()
            .map(|_| start.copy_to_ptr())
            .collect();
        // convert all 100 pointers back to RawVecs
        let raw_vecs: Vec<RawVec<u64>> = ptrs
            .iter()
            .map(|(ptr_ref, len_ref)| {
                let ptr = *ptr_ref;
                let len = *len_ref;
                unsafe { RawVec::from_ptr(ptr, len) }
            })
            .collect();
        // make sure all 100 RawVecs are equal to the starting one
        raw_vecs.iter().for_each(|raw_vec_ref| {
            assert_eq!(start, *raw_vec_ref);
        })
        // at this point the starting RawVec and all 100 RawVecs
        // will be dropped. we'll panic with a double-free if
        // memory was not actually copied
    }

    #[test]
    fn basic_round_trip_copy_from_ptr() {
        // make a pointer
        let (ptr, len): (*mut i32, usize) = RawVec::from(vec![1, 2, 3, 4, 5]).into();

        // copy from the previously created pointer 100 times
        let copied_raw_vecs: Vec<RawVec<i32>> = (0..100)
            .collect::<Vec<u32>>()
            .iter()
            .map(|_| unsafe { RawVec::copy_from_ptr(ptr, len) })
            .collect();
        // ensure each RawVec has the same contents as the original
        let orig_raw_vec = unsafe { RawVec::from_ptr(ptr, len) };
        copied_raw_vecs.iter().for_each(|raw_vec_ref| {
            assert_eq!(orig_raw_vec, *raw_vec_ref);
        });
        // at this point, orig_raw_vec and each of copied_raw_vecs
        // should point to different memory. if they don't,
        // we'll get double-frees
    }

    #[test]
    fn clone() {}

    #[test]
    fn round_trip_vec() {
        let initial_vec = vec![1, 2, 3, 4];
        let rv = RawVec::from(initial_vec.clone());
        let intermediate_vec = Vec::from(rv.clone());
        assert_eq!(initial_vec, intermediate_vec);
        let new_rv = RawVec::from(intermediate_vec.clone());
        assert_eq!(rv, new_rv);
        assert_eq!(initial_vec, Vec::from(new_rv.clone()));
        assert_eq!(intermediate_vec, Vec::from(new_rv));
    }
}
