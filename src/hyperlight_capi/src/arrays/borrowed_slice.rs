use anyhow::{bail, Result};
use std::panic::catch_unwind;

/// Borrow the memory in the range
/// `[ ptr, ptr + (len * std::mem::size_of::<T>()) )`
/// (noting the right side is exclusive) and attempt to convert it to a
/// `&mut [T]`. If that succeeded, call `run` with that `&mut [T]` and return
/// the result. Otherwise, return an `Err`. If `run` was called, the slice's
/// backing memory will not be dropped.
///
/// This function's immutable sibling, `borrow_ptr_as_slice`, should be
/// used wherever possible, rather than this.
///
/// # When to use
///
/// `borrow_ptr_as_slice_mut` should generally only be used in situations where
/// all of the following conditions hold:
///
/// - You are inside an `extern "C"` (or similar) function intended to provide
/// an API via an FFI interface
/// - Foreign code (e.g. a C program) passes you a mutable pointer and a length,
/// both of which together are intended to represent an array (most likely on
/// the heap)
/// - You intend to borrow that memory, write to it, and return it back
/// to the caller
///
/// # Use sparingly
///
/// As implied by the previous section, this function should only be used in
/// FFI scenarios. Simultaneously (or maybe consequently), it is also inherently
/// unsafe. Thus, if you use it, you'll definitely be introducing code that
/// requires you to manually do the borrow checker's work. In other words,
/// you'll be poking a hole in the borrow checker and Rust's memory safety
/// system.
///
/// Because `borrow_ptr_as_slice_mut` gives callers potentially many ways to
/// introduce memory bugs in their programs, it is intentionally as restrictive
/// as possible.
///
/// If you find yourself here intent on expanding this function's capabilities
/// or API surface, please strongly consider other options.
///
/// Also, if you're considering using this function, it's strongly recommended
/// to consider whether `RawVec` -- which takes ownership of memory rather
/// than borrowing it -- can meet your needs instead.
///
/// # Usage
///
/// If you do decide to use a this function, do so as sparingly as possible.
/// The subsections herein provide guidance on how to most effectively and
/// safely do so.
///
/// # More on the `run` function's parameter type
///
/// As shown, the `run` callback (a `FnOnce(&'a mut [T]) -> Result<U>`)
/// takes `&'a mut [T]`. In simplified terms, this parameter is a type-safe
/// "facade" over the raw underlying memory to which `ptr` points. Pay close
/// attention to the fact that it's a _reference_ with lifetime `'a`.
/// Consistent with borrowing and lifetime semantics, this parameter is
/// intended to be a valid borrow for the scope of the callback function.
///
/// If you clone the borrow, that too must not live longer than the scope of
/// the callback function (the compiler will help enforce this). It is not
/// recommended to do a deep copy; `RawVec` will likely better achieve
/// that functionality.
///
/// # Safety
///
/// `ptr` must point to the start of a contiguous memory range of size
/// `len * std::mem::size_of::<T>()`. This memory must not be mutated for the
/// duration of this function's execution, including the time during which
/// the callback is executing.
///
/// Finally, this function only borrows the memory behind `ptr`. Thus,
/// it is the caller's responsibilty to ensure that memory is dropped.
pub(crate) unsafe fn borrow_ptr_as_slice_mut<'a, T, U, F>(
    ptr: *mut T,
    len: usize,
    run: F,
) -> Result<U>
where
    F: FnOnce(&'a mut [T]) -> Result<U>,
    T: std::panic::RefUnwindSafe + 'a,
{
    match catch_unwind(|| std::slice::from_raw_parts_mut(ptr, len)) {
        Ok(slc) => run(slc),
        Err(e) => bail!(
            "borrow_ptr_as_slice_mut: panic converting (ptr, len) to slice ({:?})",
            e
        ),
    }
}

/// Equivalent to borrow_ptr_as_slice_mut, except for dealing with immutable
/// pointers and slices rather than mutable ones. This function should be used
/// rather than the mutable version wherever possible.
///
/// # Safety
///
/// The same safety concerns as borrow_ptr_as_slice_mut apply here.
pub(crate) unsafe fn borrow_ptr_as_slice<'a, T, U, F>(
    ptr: *const T,
    len: usize,
    run: F,
) -> Result<U>
where
    F: FnOnce(&'a [T]) -> Result<U>,
    T: std::panic::RefUnwindSafe + 'a,
{
    match catch_unwind(|| std::slice::from_raw_parts(ptr, len)) {
        Ok(slc) => run(slc),
        Err(e) => bail!(
            "borrow_ptr_as_slice: panic converting (ptr, len) to slice ({:?})",
            e
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{borrow_ptr_as_slice, borrow_ptr_as_slice_mut};

    #[test]
    fn simple_immut_borrow() {
        let init_vec: Vec<u64> = vec![1, 2, 3, 4, 5];
        let len = init_vec.len();
        let ptr = init_vec.as_ptr();
        unsafe {
            assert!(borrow_ptr_as_slice(ptr, len, |slc| {
                assert_eq!(len, slc.len());
                // we want to assert the underlying addresses are the same,
                // which helps us ensure no copy is made
                assert_eq!(init_vec.as_slice(), slc);
                // then, assert all the data are the same to ensure no
                // modifications are made inline
                for idx in 0..len {
                    assert_eq!(init_vec[idx], slc[idx]);
                }
                Ok(())
            })
            .is_ok());
        }
    }

    #[test]
    fn many_mut_borrows() {
        let mut init_vec: Vec<u64> = vec![1, 2, 3, 4, 5];
        let len = init_vec.len();
        let ptr = init_vec.as_mut_ptr();

        // do 100 borrows, write to each, then make sure the original init_vec
        // was modified.
        //
        // don't start this range at 0 or the first iteration will effectively
        // be a no-op. also, the values in each element of init_vec will grow
        // exponentially in this test, so don't set the upper bound of
        // this range too high or you'll run into overflows.
        (1..11).collect::<Vec<u64>>().iter().for_each(|idx| {
            let res = unsafe {
                borrow_ptr_as_slice_mut(ptr, len, |slc| {
                    // update each element of the array via slc
                    init_vec
                        .iter()
                        .enumerate()
                        .for_each(|(init_vec_idx, init_vec_elt)| {
                            let target_val: u64 = (init_vec_elt + 1) * idx;
                            // sanity check to ensure we're setting the vector's
                            // value to something new.
                            assert_ne!(*init_vec_elt, target_val);
                            slc[init_vec_idx] = target_val;
                            assert_eq!(init_vec[init_vec_idx], target_val);
                        });
                    Ok(())
                    // when slc is dropped, it should not free underlying
                    // memory and thus there should be no double-free
                })
            };
            // res will always be ok, just doing this so we don't have
            // to ignore the result of the borrow_ptr_as_slice_mut
            // call
            assert!(res.is_ok());
        });

        // when init_vec is dropped, it should be the only free of
        // actual underlying array memory and thus there should be no
        // double-free
    }
}
