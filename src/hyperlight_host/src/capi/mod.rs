#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use anyhow::{bail, Result};

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

/// A safe wrapper around an address in memory.
pub struct Addr {
    base: u64,
}

impl From<u64> for Addr {
    fn from(base: u64) -> Self {
        Addr { base }
    }
}

impl From<usize> for Addr {
    fn from(base: usize) -> Self {
        Self { base: base as u64 }
    }
}

impl Addr {
    /// Try to convert an `i64` to an instance of `Self`, or
    /// return `Err` if the conversion would be impossible.
    ///
    /// TODO: implement `TryFrom` for `i64` and use that instead of this
    pub fn from_i64(i: i64) -> Result<Self> {
        if i < 0 {
            bail!("i64 address is negative and can't be converted to an Addr");
        } else {
            Ok(Self { base: i as u64 })
        }
    }
    /// Convert `u` to an instance of `Self`.
    ///
    /// TODO: implement `TryFrom` for `usize` and use that
    /// instead of this.
    pub fn from_usize(u: usize) -> Self {
        Self { base: u as u64 }
    }

    /// Add a `usize` to `self` and return a new `Addr`
    /// instance representing the resulting addition.
    pub fn add_usize(&self, offset: usize) -> Addr {
        let new_u64 = self.base + offset as u64;
        Addr::from(new_u64)
    }

    /// Add `offset` to `self` and return a new `Addr` representing
    /// the new address after the addition.
    pub fn add_u64(&self, offset: u64) -> Addr {
        Addr::from(self.base + offset)
    }

    /// Convert `self` into a `u64`
    pub fn as_u64(&self) -> u64 {
        self.base
    }

    /// Convert `self` into an `i64`, or return `Err` if the
    /// conversion would overflow.
    ///
    /// TODO: figure out how to do this with a `TryFrom`.
    pub fn as_i64(&self) -> anyhow::Result<i64> {
        if self.base > (i64::MAX as u64) {
            bail!("Address is too large to convert to i64")
        } else {
            Ok(self.base as i64)
        }
    }

    /// Convert `self` into a `usize`, or return `Err` if the
    /// conversion would overflow.
    ///
    /// TODO: figure out how to do this with a `TryFrom`
    pub fn as_usize(&self) -> anyhow::Result<usize> {
        if self.base > usize::MAX as u64 {
            bail!("Address is too large to convert to usize")
        } else {
            Ok(self.base as usize)
        }
    }
}
