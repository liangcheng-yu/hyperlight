/// This crate contains an SDK that is used to execute specially-
/// compiled binaries within a very lightweight hypervisor environment.
#[deny(dead_code, missing_docs, unused_mut)]
/// Dealing with errors, including errors across VM boundaries
pub(crate) mod error;
/// FlatBuffers-related utilities and (mostly) generated code
#[allow(non_camel_case_types)]
pub(crate) mod flatbuffers;
/// Wrappers for host and guest functions.
#[deny(dead_code, missing_docs, unused_mut)]
pub mod func;
/// Wrapper for guest interface glue
#[deny(dead_code, missing_docs, unused_mut)]
pub mod guest_interface_glue;
/// Wrappers for hypervisor implementations
#[deny(dead_code, missing_docs, unused_mut)]
#[cfg_attr(windows, allow(dead_code))]
// TODO: Remove once HypervisorWindowsDriver is wired up outside of just tests
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
/// The main sandbox implementations. Do not use this module directly in code
/// outside this file. Types from this module needed for public consumption are
/// re-exported below.
#[deny(dead_code, missing_docs, unused_mut)]
pub mod sandbox;
/// `trait`s and other functionality for dealing with defining sandbox
/// states and moving between them
pub mod sandbox_state;
/// Utilities for testing including interacting with `simpleguest.exe`
/// and `callbackguest.exe`, our two most basic guest binaries for testing
#[deny(missing_docs, unused_mut)]
#[cfg(test)]
pub(crate) mod testing;

/// The re-export for `get_stack_boundary` function
pub use func::get_stack_boundary;
/// Re-export for `HostFunction0` trait
pub use func::HostFunction0;
/// The re-export for the `is_hypervisor_present` type
pub use sandbox::is_hypervisor_present;
/// The re-export for the `GuestBinary` type
pub use sandbox::uninitialized::GuestBinary;
/// Re-export for `CallGuestFunction` trait
pub use sandbox::CallGuestFunction;
/// Re-export for `ExecutingGuestCall` type
pub use sandbox::ExecutingGuestCall;
/// Re-export for `GuestMgr` trait
pub use sandbox::GuestMgr;
/// Re-export for `HypervisorWrapper` trait
pub use sandbox::HypervisorWrapper;
/// Re-export for `HypervisorWrapperMgr` type
pub use sandbox::HypervisorWrapperMgr;
/// Re-export for `MemMgrWrapper` type
pub use sandbox::MemMgrWrapper;
/// Re-export for `MemMgrWrapperGetter` trait
pub use sandbox::MemMgrWrapperGetter;
/// The re-export for the `Sandbox` type
pub use sandbox::Sandbox;
/// The re-export for the `SandboxRunOptions` type
pub use sandbox::SandboxRunOptions;
/// The re-export for the `UninitializedSandbox` type
pub use sandbox::UninitializedSandbox;

/// Return `Some(val)` when `cond == true`. Otherwise, return `None`
pub fn option_when<T>(val: T, cond: bool) -> Option<T> {
    match cond {
        true => Some(val),
        false => None,
    }
}
