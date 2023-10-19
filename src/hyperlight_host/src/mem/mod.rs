/// Reusable structure to hold data and provide a `Drop` implementation
pub(crate) mod custom_drop;
/// Functionality to establish a sandbox's memory layout.
pub mod layout;
/// Safe wrapper around an HINSTANCE created by the windows
/// `LoadLibrary` call
#[cfg(target_os = "windows")]
pub(crate) mod loaded_lib;
/// Functionality taht wraps a `SandboxMemoryLayout` and a
/// `SandboxMemoryConfig` to mutate a sandbox's memory as necessary.
pub mod mgr;
/// Functionality to read and mutate a PE file in a structured manner.
pub(crate) mod pe;
/// Structures to represent pointers into guest and host memory
pub mod ptr;
/// Structures to represent memory address spaces into which pointers
/// point.
pub(crate) mod ptr_addr_space;
/// Structures to represent an offset into a memory space
pub mod ptr_offset;
/// A wrapper around unsafe functionality to create and initialize
/// a memory region for a guest running in a sandbox.
pub mod shared_mem;
/// A wrapper around a `SharedMemory` and a snapshot in time
/// of the memory therein
pub mod shared_mem_snapshot;
/// Utilities for writing shared memory tests
#[cfg(test)]
pub(crate) mod shared_mem_tests;
/// An extension trait for adding types like `Offset`s to
/// types like pointers
pub(crate) mod try_add_ext;
