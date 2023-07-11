/// This crate contains an SDK that is used to execute specially-
/// compiled binaries within a very lightweight hypervisor environment.
#[deny(dead_code, missing_docs, unused_mut)]
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
/// In general, if you call a function that returns a pointer
/// to something, you must call `free_*` on that pointer when
/// you're done with the memory, unless otherwise noted.
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
#[deny(dead_code, missing_docs, unused_mut)]
pub mod capi;
/// Dealing with errors, including errors across VM boundaries
pub(crate) mod error;
/// FlatBuffers-related utilities and (mostly) generated code
#[allow(non_camel_case_types)]
pub mod flatbuffers;
/// Wrappers for host and guest functions.
#[deny(dead_code, missing_docs, unused_mut)]
pub mod func;
/// Types used to pass data to/from the guest.
#[deny(dead_code, missing_docs, unused_mut)]
pub(crate) mod guest;
/// Wrapper for guest interface glue
#[deny(dead_code, missing_docs, unused_mut)]
pub mod guest_interface_glue;
/// Wrappers for hypervisor implementations
#[deny(dead_code, missing_docs, unused_mut)]
pub mod hypervisor;
/// Functionality to establish and manage an individual sandbox's
/// memory.
///
/// The following structs are not used other than to calculate the size of the memory needed
/// and also to illustrate the layout of the memory:
///
/// - `HostFunctionDefinitions`
/// - `HostExceptionData`
/// - `GuestError`
/// - `CodeAndOutBPointers`
/// - `InputData`
/// - `OutputData`
/// - `GuestHeap`
/// - `GuestStack`
///
/// the start of the guest  memory contains the page tables and is always located at the Virtual Address 0x00200000 when
/// running in a Hypervisor:
///
/// Virtual Address
///
/// 0x200000    PML4
/// 0x201000    PDPT
/// 0x202000    PD
/// 0x203000    The guest PE code (When the code has been loaded using LoadLibrary to debug the guest this will not be
/// present and code length will be zero;
///
/// The pointer passed to the Entrypoint in the Guest application is the 0x200000 + size of page table + size of code,
/// at this address structs below are laid out in this order
#[deny(dead_code, missing_docs, unused_mut)]
pub mod mem;
/// The main sandbox implementation.
#[deny(dead_code, missing_docs, unused_mut)]
pub mod sandbox;
/// The run options for a sandbox.
#[deny(dead_code, missing_docs, unused_mut)]
pub mod sandbox_run_options;
/// `trait`s and other functionality for dealing with defining sandbox
/// states and moving between them
pub mod sandbox_state;
/// Utilities for testing including interacting with `simpleguest.exe`
/// and `callbackguest.exe`, our two most basic guest binaries for testing
#[deny(dead_code, missing_docs, unused_mut)]
#[cfg(test)]
pub(crate) mod testing;
