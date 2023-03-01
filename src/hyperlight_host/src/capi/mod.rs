#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

///! C-compatible API functions for top-level `Sandbox` objects.
pub mod api;
///! C-compatible API functions to manipulate guest and host functions.
pub mod api_funcs;
///! C-compatible functions for manipulating booleans in `Handle`s
pub mod bool;
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
///! C-compatible API functions to manage guest errors.
pub mod guest_error;
///! C-compatible API functions to manage `Handle` structures,
///! which are specialized pointers purpose-built for the Hyperlight
///! C API.
pub mod handle;
///! C-compatible API functions to introspect the status of a `Handle`
pub mod handle_status;
///! Conversion functions between `Handle` and the `Hdl` type, which
///! is a more Rust-friendly representation of a `Handle`.
pub mod hdl;
#[cfg(target_os = "linux")]
///! Provides a C API for creating and running guests on HyperV on Linux.
pub mod hyperv_linux;
///! C-compatible API functions for converting `Handle`s to various
/// integer types.
pub mod int;
#[cfg(target_os = "linux")]
///! C-compatible API functions for creating and running guests on KVM
///! on Linux.
pub mod kvm;
///! C-compatible API functions for manipulating memory access
///! handler callback functions
pub mod mem_access_handler;
///! C-compatible API functions to get basic information about
///! memory configuration
pub mod mem_cfg;
///! C-compatible API functions to manage `SandboxMemoryLayout`
///! structures.
pub mod mem_layout;
///! C-compatible API functions to manage `SandboxMemoryManager`
///! structures.
pub mod mem_mgr;
///! C-compatible API functions for manipulating outb handler callback
///! functions.
pub mod outb_handler;
///! C-compatible API functions to manage `PEInfo` structures.
pub mod pe;
///! C-compatible API functions to manage `Sandbox` structures.
pub mod sandbox;
///! C-compatible API functions to manage guest / shared memory.
pub mod shared_mem;
///! C-compatible API functions to manage guest memory snapshots
pub mod shared_mem_snapshot;
///! C-compatible types and functions, and Rust helper functions
///! for managing both Rust-native `String` types and C-style strings.
pub mod strings;
///! C-compatible API functions for converting `Handle`s to various
/// unsigned integer types.
pub mod uint;
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
