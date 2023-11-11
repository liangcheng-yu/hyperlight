/// This crate contains an SDK that is used to execute specially-
/// compiled binaries within a very lightweight hypervisor environment.
#[deny(dead_code, missing_docs, unused_mut)]
/// Dealing with errors, including errors across VM boundaries
pub mod error;
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
/// Metric definitions and helpers
#[deny(dead_code, missing_docs, unused_mut)]
pub mod metrics;
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

/// The re-export for the `HyperlightError` type
pub use error::HyperlightError;
/// The re-export for `get_stack_boundary` function
pub use func::get_stack_boundary;
/// Re-export for `HostFunction0` trait
pub use func::HostFunction0;
/// The re-export for the set_registry function
pub use metrics::set_metrics_registry;
/// The re-export for the `is_hypervisor_present` type
pub use sandbox::is_hypervisor_present;
/// The re-export for the `GuestBinary` type
pub use sandbox::uninitialized::GuestBinary;
/// Re-export for `HypervisorWrapper` trait
pub use sandbox::HypervisorWrapper;
/// Re-export for `MemMgrWrapper` type
pub use sandbox::MemMgrWrapper;
/// A sandbox that can call be used to make multiple calls to guest functions,
/// and otherwise reused multiple times
pub use sandbox::MultiUseSandbox;
/// The re-export for the `SandboxRunOptions` type
pub use sandbox::SandboxRunOptions;
/// A sandbox that can be used at most once to call a guest function, and
/// then must be discarded.
pub use sandbox::SingleUseSandbox;
/// The re-export for the `UninitializedSandbox` type
pub use sandbox::UninitializedSandbox;
/// Return `Some(val)` when `cond == true`. Otherwise, return `None`
pub fn option_when<T>(val: T, cond: bool) -> Option<T> {
    match cond {
        true => Some(val),
        false => None,
    }
}
/// The universal `Result` type used throughout the Hyperlight codebase.
pub type Result<T> = core::result::Result<T, error::HyperlightError>;

// Logs an error then returns with it , more or less equivalent to the bail! macro in anyhow
// but for HyperlightError instead of anyhow::Error
#[macro_export]
macro_rules! log_then_return {
    ($msg:literal $(,)?) => {{
        let __args = std::format_args!($msg);
        let __err_msg = match __args.as_str() {
            Some(msg) => String::from(msg),
            None => std::format!($msg),
        };
        let __err = $crate::HyperlightError::Error(__err_msg);
        log::error!("{}", __err);
        return Err(__err);
    }};
    ($err:expr $(,)?) => {
        log::error!("{}", $err);
        return Err($err);
    };
    ($err:stmt $(,)?) => {
        log::error!("{}", $err);
        return Err($err);
    };
    ($fmtstr:expr, $($arg:tt)*) => {
           let __err_msg = std::format!($fmtstr, $($arg)*);
           let __err = $crate::error::HyperlightError::Error(__err_msg);
           log::error!("{}", __err);
           return Err(__err);
    };
}
