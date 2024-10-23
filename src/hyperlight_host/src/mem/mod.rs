/// Reusable structure to hold data and provide a `Drop` implementation
#[cfg(inprocess)]
pub(crate) mod custom_drop;
/// A simple ELF loader
pub(crate) mod elf;
/// A generic wrapper for executable files (PE, ELF, etc)
pub(crate) mod exe;
/// Functionality to establish a sandbox's memory layout.
pub mod layout;
/// Safe wrapper around an HINSTANCE created by the windows
/// `LoadLibrary` call
#[cfg(target_os = "windows")]
pub(super) mod loaded_lib;
/// memory regions to be mapped inside a vm
pub mod memory_region;
/// Functionality taht wraps a `SandboxMemoryLayout` and a
/// `SandboxMemoryConfig` to mutate a sandbox's memory as necessary.
pub mod mgr;
/// Functionality to read and mutate a PE file in a structured manner.
pub(crate) mod pe;
/// Structures to represent pointers into guest and host memory
pub mod ptr;
/// Structures to represent memory address spaces into which pointers
/// point.
pub(super) mod ptr_addr_space;
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
