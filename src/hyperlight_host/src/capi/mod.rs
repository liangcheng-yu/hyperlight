///! A C-compatible API for the Hyperlight Host's Sandbox and
///! associated types.
///!
///! Because C is an unmanaged language without facilities to
///! otherwise help in releasing memory (like Rust's ownership
///! or C++'s destructors), the rules for how to handle memory
///! are written on most of the functions herein.
///!
///! Also, all of the data structures in the header file that are
///! forward-declared (e.g. `struct`s that are named but not
///! defined) are intended to be opaque. Do not attempt to
///! manipulate them directly. Instead, treat them as if they
///! were a `void` or `void *` type and use only the provided
///! functions to manipulate them.
///!
///! In general, if you call a function that returns a pointer
///! to something, you must call `free_*` on that pointer when
///! you're done with the memory, unless otherwise noted.
///!
///! # Examples
///!
///! You should first create a new `Context` and `Sandbox`:
///!
///! ```
///! Context* ctx = context_new();
///! Handle sbox_hdl = sandbox_new(ctx, "binary_to_run_inside_VM");
///! ```
///!
///! ... and then you can call functions on it to get the
///! state of the system:
///!
///! ```
///! Handle path_hdl = guest_binary_path(ctx, sbox_hdl);
///! ```
///!
///! ... and then you must clean up the memory you've created
///! along the way:
///!
///! ```
///! handle_free(path_hdl);
///! handle_free(sbox_hdl);
///! context_free(ctx);
///! ```
pub mod api;
pub mod api_funcs;
pub mod byte_array;
pub mod callback;
pub mod context;
pub mod err;
pub mod filepath;
pub mod handle;
pub mod hdl;
pub mod mem_config;
pub mod mem_layout;
pub mod pe;
pub mod sandbox;
pub mod strings;
pub mod val_ref;

/// Create a `Vec<i8>` from all the memory from `arr_ptr` to `arr_ptr + arr_len`
/// inclusive of `arr_len`.
///
/// # Safety
///
/// `arr_ptr` must point to the start of a memory block you exclusively own,
/// and you must own all the memory from `arr_ptr` through `arr_ptr + arr_len`
/// inclusive. Ensure that no part of this memory is modified while this
/// function is executing.
///
/// This function makes a _copy_ of the memory you pass, so make sure you
/// clean it up afterward.
unsafe fn fill_vec<T>(arr_ptr: *const T, arr_len: usize) -> Vec<T> {
    let mut vec = Vec::<T>::with_capacity(arr_len);
    std::ptr::copy(arr_ptr, vec.as_mut_ptr(), arr_len);
    vec.set_len(arr_len);
    vec
}
