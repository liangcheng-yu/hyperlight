///! C-compatible API functions for top-level `Sandbox` objects.
pub mod api;
///! C-compatible API functions to manipulate guest and host functions.
pub mod api_funcs;
///! C-compatible API functions to manage plain byte arrays.
pub mod byte_array;
///! C-compatible API functions to create, modify, read and delete
///! C language function pointers.
///!
///! Most often used to create guest functions.
pub mod callback;
///! C-compatible API functions to manage `Context` objects, which
///! are used as a specialized memory store for the Hyperlight C API.
pub mod context;
///! C-compatible API functions to manage errors in the Hyperlight
///! C API.
pub mod err;
///! C-compatible APIs to manipulate paths to files.
pub mod filepath;
///! C-compatible API functions to manage `Handle` structures,
///! which are specialized pointers purpose-built for the Hyperlight
///! C API.
pub mod handle;
///! Conversion functions between `Handle` and the `Hdl` type, which
///! is a more Rust-friendly representation of a `Handle`.
pub mod hdl;
///! C-compatible API functions to manage `SandboxMemoryConfiguration`
///! structures.
pub mod mem_config;
///! C-compatible API functions to manage `SandboxMemoryLayout`
///! structures.
pub mod mem_layout;
///! C-compatible API functions to manage `PEInfo` structures.
pub mod pe;
///! C-compatible API functions to manage `Sandbox` structures.
pub mod sandbox;
///! C-compatible types and functions, and Rust helper functions
///! for managing both Rust-native `String` types and C-style strings.
pub mod strings;
///! C-compatible API functions for managing `Val` structures.
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
