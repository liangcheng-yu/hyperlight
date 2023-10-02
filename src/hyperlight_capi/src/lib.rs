/// A C-compatible API for the Hyperlight Host's Sandbox and
/// associated types.
///
/// Because C is an unmanaged language without facilities to
/// otherwise help in releasing memory (like Rust's ownership
/// or C++'s destructors), the rules for how to handle memory
/// are written on most of the functions herein.
///
/// Also, all of the data structures in the header file that are
/// forward-declared (e.g. `struct`s that are named but not
/// defined) are intended to be opaque. Do not attempt to
/// manipulate them directly. Instead, treat them as if they
/// were a `void` or `void *` type and use only the provided
/// functions to manipulate them.
///
/// In general, there are very few functions that return a pointer to something,
/// but if you call one, it has created memory that you now own. You must use
/// the corresponding `free_*` function, if one exists, to free that memory when you're done
/// with it. If such a `free_*` function does not exist, you should the built-in
/// `free` function.
///
/// Most functions in this API return a `Handle`, which is a safe(r) reference to
/// memory that lives in the `Context` you passed to that function. When you're
/// done with that memory, you must call `handle_free`, passing the original
/// `Context` and the returned `Handle`.
///
/// Finally, when you are done with the `Context`, you must pass it to a _single_ call
/// to `context_free`. Failing to do so will result in memory leaks. Doing so multiple
/// times will result in double-free errors. It's most common to create a
/// new `Context` at the beginning of your program, use it throughout,
/// then free it once at the end.
///
/// # Examples
///
/// You should first create a new `Context` and `Sandbox`:
///
/// ```
/// Context* ctx = context_new();
/// Handle sbox_hdl = sandbox_new(ctx, "binary_to_run_inside_VM");
/// ```
///
/// ... and then you can call functions on it to get the
/// state of the system:
///
/// ```
/// Handle path_hdl = guest_binary_path(ctx, sbox_hdl);
/// ```
///
/// ... and then you must clean up the memory you've created
/// along the way:
///
/// ```
/// handle_free(path_hdl);
/// handle_free(sbox_hdl);
/// context_free(ctx);
/// ```
/// C-compatible adapters for converting `Vec`s to/from raw pointers
pub mod arrays;
/// C-compatible functions for manipulating booleans in `Handle`s
pub mod bool;
/// C-compatible API functions to manage plain byte arrays.
pub mod byte_array;
/// Wrapper utility for creating efficient and correct FFI functions
pub(crate) mod c_func;
/// C-compatible API functions to get basic information about
/// configuration
pub mod config;
/// C-compatible API functions to manage `Context` objects, which
/// are used as a specialized memory store for the Hyperlight C API.
pub mod context;
/// C-compatible API functions to manage errors in the Hyperlight
/// C API.
pub mod err;
/// C-compatible functions to be exported to guests
pub mod exports;
/// C-compatible APIs to manipulate paths to files.
pub mod filepath;
/// C-compatible APIs to manage function_call_results.
pub mod function_call_result;
/// C-compatible API functions to manage guest errors.
pub mod guest_error;
/// C-compatible API function for the guest glue interface.
pub mod guest_interface_glue;
/// C-compatible API functions to manipulate `GuestLogData`s
pub mod guest_log_data;
/// C-compatible API functions to manage `Handle` structures,
/// which are specialized pointers purpose-built for the Hyperlight
/// C API.
pub mod handle;
/// C-compatible API functions to introspect the status of a `Handle`
pub mod handle_status;
/// Conversion functions between `Handle` and the `Hdl` type, which
/// is a more Rust-friendly representation of a `Handle`.
pub mod hdl;
/// C-compatible API functions to manage host function calls.
pub mod host_function_call;
#[cfg(target_os = "linux")]
/// Provides a C API for creating and running guests on HyperV on Linux.
pub mod hyperv_linux;
/// C-compatible API functions for converting `Handle`s to various
/// integer types.
pub mod int;
#[cfg(target_os = "linux")]
/// Provides a C API for creating and running guests on KVM on Linux.
pub mod kvm;
/// C-compatible API functions for manipulating memory access
/// handler callback functions
pub mod mem_access_handler;
/// C-compatible API functions to manage `SandboxMemoryLayout`
/// structures.
pub mod mem_layout;
/// C-compatible API functions to manage `SandboxMemoryManager`
/// structures.
pub mod mem_mgr;
/// C-compatible API functions for manipulating outb handler callback
/// functions.
pub mod outb_handler;
/// C-compatible API functions to manage `Sandbox` structures.
pub mod sandbox;
/// C-compatible types and functionality for wrapping the rust-native
/// `*Sandbox` types
pub mod sandbox_compat;
/// C-compatible API functions to manage guest / shared memory.
pub mod shared_mem;
/// C-compatible API functions to manage guest memory snapshots
pub mod shared_mem_snapshot;
/// C-compatible types and functions, and Rust helper functions
/// for managing both Rust-native `String` types and C-style strings.
pub mod strings;
/// C-compatible API functions for converting `Handle`s to various
/// unsigned integer types.
pub mod uint;
