#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

///! C-compatible API functions for top-level `Sandbox` objects.
pub mod api;
///! C-compatible API functions to manipulate guest and host functions.
pub mod api_funcs;
///! C-compatible adapters for converting `Vec`s to/from raw pointers
pub mod arrays;
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
///! C-compatible functions to be exported to guests
pub mod exports;
///! C-compatible APIs to manipulate paths to files.
pub mod filepath;
///! C-compatible APIs to manage function_call_results.
pub mod function_call_result;
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
///! C-compatible API functions to manage host function calls.
pub mod host_function_call;
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

/// Return `Some(val)` when `cond == true`. Otherwise, return `None`
pub(crate) fn option_when<T>(val: T, cond: bool) -> Option<T> {
    match cond {
        true => Some(val),
        false => None,
    }
}
