use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use crate::{uint::register_u64, validate_context, validate_context_or_panic};
use hyperlight_host::mem::shared_mem::SharedMemory;
use hyperlight_host::{new_error, Result};

mod impls {
    use crate::{byte_array::get_byte_array, context::Context, handle::Handle};
    use hyperlight_host::mem::shared_mem::SharedMemory;
    use hyperlight_host::{log_then_return, Result};
    use std::cell::RefCell;

    /// Get the starting address of the shared memory in `ctx` referenced by `hdl`
    pub(crate) fn get_address(ctx: &Context, hdl: Handle) -> Result<usize> {
        let shared_mem = super::get_shared_memory(ctx, hdl)?;
        Ok(shared_mem.raw_ptr() as usize)
    }

    /// Look up the `[u8]` referenced by `byte_arr_hdl` in `ctx`,
    /// get the slice in the range `[arr_start, arr_start + arr_length)`,
    /// wrap that slice in a `RefCell`, and return it
    pub(crate) fn copy_from_byte_array(
        ctx: &Context,
        byte_arr_hdl: Handle,
        arr_start: usize,
        arr_length: usize,
    ) -> Result<RefCell<&[u8]>> {
        let byte_arr = get_byte_array(ctx, byte_arr_hdl)?;
        let byte_arr_len = (*byte_arr).len();

        // ensure we're not starting off the end of the byte array
        if arr_start >= byte_arr_len {
            log_then_return!("Array start ({}) is out of bounds", arr_start);
        }

        // ensure we're not ending off the end of the byte array
        let arr_end = arr_start + arr_length;
        if (arr_start + arr_length) > byte_arr_len {
            log_then_return!("Array end ({}) is out of bounds", arr_end);
        }

        // get the slice of byte_arr.
        // slice semantics give a byte_arr in the range: [arr_start, arr_end)
        // (i.e. inclusive of arr_start, exclusive of arr_end)
        let slice = &(*byte_arr)[arr_start..arr_end];

        Ok(RefCell::new(slice))
    }

    /// Attempt to look up the `SharedMemory` referenced by `hdl` in `ctx`,
    /// then if one exists, return it wrapped in a `RefCell`.
    ///
    /// Returns `Err` if no such `SharedMemory` exists.
    ///
    /// This function is useful because you must have access to a
    /// `&mut Context` if you want to do mutable operations on
    /// a `SharedMemory` stored therein.
    /// Instead, when you need to do mutable operations on a `SharedMemory`,
    /// pass a `&Context` (immutable reference) to this function, then
    /// call `try_borrow_mut` on the resulting `RefCell`
    ///
    /// # Example
    ///
    /// ```rust
    /// // assume we have a Context called ctx and a Handle to
    /// // a valid SharedMemory in Context, called hdl
    /// let shared_mem_refcell: RefCell<SharedMemory> = get_shared_memory_ref(ctx, hdl).unwrap();
    /// let shared_mem_ref: RefMut<SharedMemory> = res.try_borrow_mut().unwrap();
    /// let shared_mem_mut: &mut SharedMemory = *shared_mem_ref;
    /// ```
    fn get_shared_memory_ref(ctx: &Context, hdl: Handle) -> Result<RefCell<SharedMemory>> {
        let gm = super::get_shared_memory(ctx, hdl)?;
        // ok to clone the SharedMemory here because internally, it's just
        // a reference-counted pointer, so we're simply incrementing the
        // reference count. Memory won't be deleted until all clones and the
        // original go out of scope. see documentation inside SharedMemory
        // for more detail
        Ok(RefCell::new(gm.clone()))
    }

    /// Copy all values in the byte array referenced by `byte_array_hdl`,
    /// in the range `[arr_start, arr_start + arr_length)` into the
    /// `SharedMemory` referenced by `shared_mem_hdl`
    pub(crate) fn copy_byte_array(
        ctx: &mut Context,
        shared_mem_hdl: Handle,
        byte_array_hdl: Handle,
        shared_mem_offset: usize,
        arr_start: usize,
        arr_length: usize,
    ) -> Result<()> {
        // Below is a comprehensive explanation of why we're using
        // `RefCell` below to fetch and access the byte array and shared memory.
        // I'm including it because it took me (arschles) a long time to
        // figure out the best way to get `RefCell` working properly. If you
        // intend to change something inside this function, you should probably
        // read at least until the "stfu borrow checker" part of this comment.
        //
        // # Background on the problem
        //
        // To start, here's a description of the problem we're facing
        // in this function. W
        //
        // We have to fetch two things from `ctx`:
        //
        // 1. The `Vec<u8>` referenced by `byte_array_hdl`, immutably
        // 2. The `SharedMemory` referenced by `shared_mem_offset`, mutably
        //
        // In other words, we're only going to read from the `Vec<u8>`
        // in (1), but we're going to write to the `SharedMemory` in (2).
        //
        // So, to do (1), we have to borrow `ctx` immutably and to do (2)
        // we have to borrow `ctx` mutably. This arrangement violates
        // the borrow checker. We can't copy `ctx` to get around this
        // problem, because that violates the borrow checker rules.
        // (i.e. if you borrow anything mutably, as in (2), you can't borrow
        // anything else, mutably or immutably, as in (1))
        //
        // Of course, we know that this isn't going to be a problem
        // in reality because we're not going to be reading any parts of `ctx`
        // that we're also mutating. In fact, the read -- of the `Vec<u8>`
        // -- happens strictly before the write to the `SharedMemory`.
        //
        // # How `RefCell` helps us solve the problem
        //
        // We don't have a clean way to indicate to the borrow checker
        // that, essentially, we know what we're doing. At the end of the
        // day, you need to pass a `&mut Context` to get a `&mut SharedMemory`,
        // and that means you can't pass a `&mut Context` or a `&Context`
        // anywhere else within that same scope. Also, the borrow checker
        // is smart enough to know that _any_ reference you got from
        // that `&mut Context`, which escapes the scope, could also mutate
        // the `Context` and needs to have exclusive access.
        //
        // All this is to say there may be a very complex way to tell the
        // borrow checker we know what we're doing, or to trick the borrow
        // checker, but it's not worth doing because we have a very well
        // defined and relatively simple way to do the same thing built
        // into the standard library. Read on for more.
        //
        // # Enter `RefCell`
        //
        // `RefCell` is Rust's built-into-the-standard-library way to
        // tell the borrow checker we know what we're doing with respect
        // to mutability. In other words, we can break the exclusive access
        // rules in a well-defined, somewhat-safe way.
        //
        // `RefCell` docs call this somewhat-safe way to break the rules
        // "interior mutability".
        //
        // In the below code, `RefCell` is allowing us to pass a
        // `&Context` to some code that gives us back a `RefCell<SharedMemory>`.
        // We can then, in turn, use this `RefCell` to mutate the contained
        // `SharedMemory.
        //
        // In fun terms, our end goal is to say "stfu borrow checker,
        // I know what I'm doing"
        //
        // >If you want to dive into more details, read on.
        // >Otherwise, you can stop reading.
        //
        // # More about `RefCell`
        //
        // As said above, `RefCell` is how we get around the borrow checker's
        // exclusive mutating access rule. The standard library calls this
        // feature "interior mutability" - outwardly to the borrow checker,
        // you can't mutate the `RefCell`, but if you reach inside to the
        // _interior_ of that `RefCell`, you can mutate it.
        //
        // Again, in fun terms: `interior mutability = "stfu borrow checker"`
        //
        // Recall above that we had to read the `SharedMemory` from `ctx`,
        // but since we're going to fetch that `SharedMemory` for mutation,
        // we had to borrow `ctx` mutably, and that caused the borrow checker
        // to (rightfully) cause a compile error.
        //
        // `RefCell` is precisely what allows us to borrow `ctx` immutably to
        // get the `SharedMemory` we need, and then later allow us to mutate
        // that `SharedMemory` anyway. See that in action in the call below to
        // `get_shared_memory_ref`. In that call, we're passing `ctx` in as
        // a `&Context` -- an immutable reference.
        //
        // That function, in turn, returns a `Result<RefCell<SharedMemory>>`,
        // but let's ignore that outer `Result` here for simplicity. Once we
        // have our `RefCell<SharedMemory>`, we have several useful methods we
        // can call.
        //
        // Since at the end of the day, we want a `&mut SharedMemory`, the one
        // we care about most is `try_borrow_mut`. That function gives us
        // a `Result<RefMut<SharedMemory>>`. Here, that outer `Result` matters
        // because if it returns an `Err`, that means someone else has called
        // `try_borrow_mut` before us. This function is how `RefCell` does
        // borrow checking at runtime, and allowing us to quiet the borrow
        // checker at compile time.
        let data = {
            let data_refcell = copy_from_byte_array(ctx, byte_array_hdl, arr_start, arr_length)?;
            let data_ref = data_refcell.try_borrow()?;
            *data_ref
        };
        let shared_mem = &mut {
            let gm_refcell = get_shared_memory_ref(ctx, shared_mem_hdl)?;
            let gm_refmut = gm_refcell.try_borrow_mut()?;
            // Note: this clone is cheap. It just increments a reference-counter
            // inside the SharedMemory. See docs on SharedMemory for more
            // information
            (*gm_refmut).clone()
        };

        shared_mem.copy_from_slice(data, shared_mem_offset)
    }
}

/// Get the `SharedMemory` stored in `ctx` and referenced by `hdl` and return
/// it inside a `ReadResult` suitable only for read operations.
///
/// Returns `Ok` if `hdl` is a valid `SharedMemory` in `ctx`,
/// `Err` otherwise.
pub(crate) fn get_shared_memory(ctx: &Context, hdl: Handle) -> Result<&SharedMemory> {
    Context::get(hdl, &ctx.shared_mems, |g| matches!(g, Hdl::SharedMemory(_)))
}

/// Store the given `shared_mem` in `ctx` and return a new `Handle`
/// referencing it.
pub(crate) fn register_shared_mem(ctx: &mut Context, shared_mem: SharedMemory) -> Handle {
    Context::register(shared_mem, &mut ctx.shared_mems, Hdl::SharedMemory)
}

/// Get the starting address of the shared memory referenced
/// by `hdl` in `ctx`, or `0` if the handle is invalid.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn shared_memory_get_address(ctx: *const Context, hdl: Handle) -> usize {
    validate_context_or_panic!(ctx);

    impls::get_address(&*ctx, hdl).unwrap_or(0)
}

/// Get the size of the shared memory in `ctx` referenced by `hdl`, then
/// return a new `Handle` referencing the `uint64` size in `ctx`.
///
/// If there was any error, return a `Handle` referencing that error in `ctx`
/// instead.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn shared_memory_get_size(ctx: *mut Context, hdl: Handle) -> Handle {
    validate_context_or_panic!(ctx);
    let sm = match get_shared_memory(&*ctx, hdl) {
        Ok(s) => s,
        Err(e) => return (*ctx).register_err(e),
    };
    let size = match u64::try_from(sm.raw_mem_size()) {
        Ok(s) => s,
        Err(_) => {
            return (*ctx).register_err(new_error!(
                "shared_memory_get_size: couldn't convert usize memory size value ({:?}) to u64",
                sm.raw_mem_size()
            ))
        }
    };
    register_u64(&mut *ctx, size)
}

/// Fetch the following two strutures:
/// * The byte array in `ctx` referenced by `byte_array_hdl`
/// * The shared memory in `ctx` referenced by `shared_mem_hdl`
///
/// ... then copy the data from the byte array in the range
/// `[arr_start, arr_start + arr_length)` (i.e. the left side is
/// inclusive and the right side is not inclusive) into the shared
/// memory starting at offset `offset`.
///
/// Return an empty `Handle` if both the shared memory and byte array
/// were found and the copy succeeded, and an error handle otherwise.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn shared_memory_copy_from_byte_array(
    ctx: *mut Context,
    shared_mem_hdl: Handle,
    byte_array_hdl: Handle,
    offset: usize,
    arr_start: usize,
    arr_length: usize,
) -> Handle {
    validate_context!(ctx);

    match impls::copy_byte_array(
        &mut *ctx,
        shared_mem_hdl,
        byte_array_hdl,
        offset,
        arr_start,
        arr_length,
    ) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::impls::copy_byte_array;
    use super::register_shared_mem;
    use crate::{context::Context, handle::Handle, hdl::Hdl};
    use hyperlight_host::{mem::shared_mem::SharedMemory, Result};

    struct TestData {
        // Context used to create all handles herein
        pub(crate) ctx: Box<Context>,
        // Handle to shared memory
        pub(crate) shared_mem_hdl: Handle,
        // Size of shared memory
        pub(crate) shared_mem_size: usize,
        // Handle to byte array
        pub(crate) byte_arr_hdl: Handle,
        // length of byte array
        pub(crate) barr_len: usize,
    }

    impl TestData {
        pub(crate) fn new(barr_vec_len: usize, shared_mem_size: usize) -> Result<Self> {
            let mut ctx = Context::default();
            let barr_vec = {
                let mut v = Vec::new();
                for i in 0..barr_vec_len {
                    v.push(i as u8);
                }
                v
            };
            let barr_hdl = Context::register(barr_vec, &mut ctx.byte_arrays, Hdl::ByteArray);
            let shared_mem_hdl = {
                let sm = SharedMemory::new(shared_mem_size).unwrap();
                register_shared_mem(&mut ctx, sm)
            };
            Ok(Self {
                ctx: Box::new(ctx),
                shared_mem_hdl,
                shared_mem_size,
                byte_arr_hdl: barr_hdl,
                barr_len: barr_vec_len,
            })
        }
    }

    #[test]
    fn copy_byte_array_at_start() {
        // copy an entire byte array into shared memory
        let mut test_data = TestData::new(3, 0x1000).unwrap();
        copy_byte_array(
            test_data.ctx.as_mut(),
            test_data.shared_mem_hdl,
            test_data.byte_arr_hdl,
            0,
            0,
            test_data.barr_len,
        )
        .unwrap();
    }

    #[test]
    fn copy_byte_array_twice() {
        let mut test_data = TestData::new(3, 0x1000).unwrap();
        copy_byte_array(
            test_data.ctx.as_mut(),
            test_data.shared_mem_hdl,
            test_data.byte_arr_hdl,
            0,
            0,
            test_data.barr_len,
        )
        .unwrap();
        copy_byte_array(
            test_data.ctx.as_mut(),
            test_data.shared_mem_hdl,
            test_data.byte_arr_hdl,
            0,
            0,
            test_data.barr_len,
        )
        .unwrap();
    }

    #[test]
    fn copy_byte_array_at_end() {
        // copy byte array to the very end of shared memory
        let mut test_data = TestData::new(3, 0x1000).unwrap();
        copy_byte_array(
            test_data.ctx.as_mut(),
            test_data.shared_mem_hdl,
            test_data.byte_arr_hdl,
            test_data.shared_mem_size - test_data.barr_len - 1,
            0,
            test_data.barr_len,
        )
        .unwrap();
    }

    #[test]
    fn copy_byte_array_invalid_offset() {
        // copy the same small byte array to an invalid offset.
        let mut test_data = TestData::new(3, 0x1000).unwrap();

        let res = copy_byte_array(
            test_data.ctx.as_mut(),
            test_data.shared_mem_hdl,
            test_data.byte_arr_hdl,
            test_data.shared_mem_size,
            0,
            1,
        );

        assert!(res.is_err());
    }

    #[test]
    fn copy_byte_array_too_much() {
        // copy too much of the small byte array
        let mut test_data = TestData::new(3, 0x1000).unwrap();
        let res = copy_byte_array(
            test_data.ctx.as_mut(),
            test_data.shared_mem_hdl,
            test_data.byte_arr_hdl,
            0,
            0,
            test_data.barr_len * 10,
        );
        assert!(res.is_err());
    }
}
