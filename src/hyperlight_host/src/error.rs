use std::array::TryFromSliceError;
use std::cell::{BorrowError, BorrowMutError};
use std::convert::Infallible;
use std::error::Error;
use std::num::TryFromIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::{MutexGuard, PoisonError};
use std::time::SystemTimeError;

#[cfg(target_os = "windows")]
use crossbeam_channel::{RecvError, SendError};
use flatbuffers::InvalidFlatbuffer;
use hyperlight_common::flatbuffer_wrappers::function_types::{ParameterValue, ReturnValue};
use hyperlight_common::flatbuffer_wrappers::guest_error::ErrorCode;
use hyperlight_common::flatbuffers::hyperlight::generated::{
    FunctionCallType, ParameterType, ParameterValue as FBParameterValue, ReturnType,
    ReturnValue as FBReturnValue,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

use crate::mem::memory_region::MemoryRegionFlags;
use crate::mem::ptr::RawPtr;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub(crate) struct HyperlightHostError {
    pub(crate) message: String,
    pub(crate) source: String,
}

/// The error type for Hyperlight operations
#[derive(Error, Debug)]
pub enum HyperlightError {
    /// Memory access out of bounds
    #[error("Offset: {0} out of bounds, Max is: {1}")]
    BoundsCheckFailed(u64, usize),

    /// Call entry point was callled when not in process
    #[error("Call_entry_point is only available with in-process mode")]
    CallEntryPointIsInProcOnly(),

    /// Checked Add Overflow
    #[error("Couldnt add offset to base address. Offset: {0}, Base Address: {1}")]
    CheckedAddOverflow(u64, u64),

    /// Cross beam channel receive error
    #[error("{0:?}")]
    #[cfg(target_os = "windows")]
    CrossBeamReceiveError(#[from] RecvError),

    /// Cross beam channel send error
    #[error("{0:?}")]
    #[cfg(target_os = "windows")]
    CrossBeamSendError(#[from] SendError<HANDLE>),

    /// CString conversion error
    #[error("Error converting CString {0:?}")]
    CStringConversionError(#[from] std::ffi::NulError),

    /// Disallowed syscall was caught
    #[error("Seccomp filter Killed Thread on disallowed syscall")]
    #[cfg(all(feature = "seccomp", target_os = "linux"))]
    DisallowedSyscall(),

    /// A generic error with a message
    #[error("{0}")]
    Error(String),

    /// Exception Data Length is incorrect
    #[error("Exception Data Length is incorrect. Expected: {0}, Actual: {1}")]
    ExceptionDataLengthIncorrect(i32, usize),

    /// Exception Message is too big
    #[error("Exception Message is too big. Max Size: {0}, Actual: {1}")]
    ExceptionMessageTooBig(usize, usize),

    /// Execution violation
    #[error("Non-executable address {0:#x} tried to be executed")]
    ExecutionAccessViolation(u64),

    /// Guest execution was cancelled by the host
    #[error("Execution was cancelled by the host.")]
    ExecutionCanceledByHost(),

    /// Accessing the value of a flatbuffer parameter failed
    #[error("Failed to get a value from flat buffer parameter")]
    FailedToGetValueFromParameter(),

    ///Field Name not found in decoded GuestLogData
    #[error("Field Name {0} not found in decoded GuestLogData")]
    FieldIsMissingInGuestLogData(String),

    /// Guest aborted during outb
    #[error("Guest aborted: {0} {1}")]
    GuestAborted(u8, String),

    ///Cannot run from guest binary unless the binary is a file
    #[error("Cannot run from guest binary when guest binary is a buffer")]
    GuestBinaryShouldBeAFile(),

    /// Guest call resulted in error in guest
    #[error("Guest error occurred {0:?}: {1}")]
    GuestError(ErrorCode, String),

    /// An attempt to cancel guest execution failed because it is hanging on a host function call
    #[error("Guest execution hung on the execution of a host function call")]
    GuestExecutionHungOnHostFunctionCall(),

    /// Guest call already in progress
    #[error("Guest call is already in progress")]
    GuestFunctionCallAlreadyInProgress(),

    /// The given type is not supported by the guest interface.
    #[error("Unsupported type: {0}")]
    GuestInterfaceUnsupportedType(String),

    /// The guest offset is invalid.
    #[error("The guest offset {0} is invalid.")]
    GuestOffsetIsInvalid(usize),

    /// A Host function was called by the guest but it was not registered.
    #[error("HostFunction {0} was not found")]
    HostFunctionNotFound(String),

    /// An attempt to communicate with or from the Hypervisor Handler thread failed
    /// (i.e., usually a failure call to `.send()` or `.recv()` on a message passing
    /// channel)
    #[error("Communication failure with the Hypervisor Handler thread")]
    HypervisorHandlerCommunicationFailure(),

    /// An attempt to cancel a Hypervisor Handler execution failed.
    /// See `terminate_hypervisor_handler_execution_and_reinitialise`
    /// for more details.
    #[error("Hypervisor Handler execution cancel attempt on a finished execution")]
    HypervisorHandlerExecutionCancelAttemptOnFinishedExecution(),

    /// A Receive for a Hypervisor Handler Message Timedout
    #[error("Hypervisor Handler Message Receive Timedout")]
    HypervisorHandlerMessageReceiveTimedout(),

    /// Reading Writing or Seeking data failed.
    #[error("Reading Writing or Seeking data failed {0:?}")]
    IOError(#[from] std::io::Error),

    /// Failed to convert to Integer
    #[error("Failed To Convert Size to usize")]
    IntConversionFailure(#[from] TryFromIntError),

    /// The flatbuffer is invalid
    #[error("The flatbuffer is invalid")]
    InvalidFlatBuffer(#[from] InvalidFlatbuffer),

    /// The function call type is invalid
    #[error("The function call type is invalid {0:?}")]
    InvalidFunctionCallType(FunctionCallType),

    /// Conversion of str to Json failed
    #[error("Conversion of str data to json failed")]
    JsonConversionFailure(#[source] serde_json::Error),

    /// Error occurred in KVM Operation
    #[error("KVM Error {0:?}")]
    #[cfg(target_os = "linux")]
    KVMError(#[source] kvm_ioctls::Error),

    /// An attempt to get a lock from a Mutex failed.
    #[error("Unable to lock resource")]
    LockAttemptFailed(String),

    /// Memory Access Violation at the given address. The access type and memory region flags are provided.
    #[error("Memory Access Violation at address {0:#x} of type {1}, but memory is marked as {2}")]
    MemoryAccessViolation(u64, MemoryRegionFlags, MemoryRegionFlags),

    /// Memory Allocation Failed.
    #[error("Memory Allocation Failed with OS Error {0:?}.")]
    MemoryAllocationFailed(Option<i32>),

    /// Memory Protection Failed
    #[error("Memory Protection Failed with OS Error {0:?}.")]
    MemoryProtectionFailed(Option<i32>),

    /// The memory request exceeds the maximum size allowed
    #[error("Memory requested {0} exceeds maximum size allowed {1}")]
    MemoryRequestTooBig(usize, usize),

    /// Metric Not Found.
    #[error("Metric Not Found {0:?}.")]
    MetricNotFound(&'static str),

    /// mmap Failed.
    #[error("mmap failed with os error {0:?}")]
    MmapFailed(Option<i32>),

    /// mprotect Failed.
    #[error("mprotect failed with os error {0:?}")]
    MprotectFailed(Option<i32>),

    /// mshv Error Occurred
    #[error("mshv Error {0:?}")]
    #[cfg(target_os = "linux")]
    MSHVError(#[from] mshv_ioctls::MshvError),

    /// No Hypervisor was found for Sandbox.
    #[error("No Hypervisor was found for Sandbox")]
    NoHypervisorFound(),

    /// Restore_state called with no valid snapshot
    #[error("Restore_state called with no valid snapshot")]
    NoMemorySnapshot,

    /// An error occurred handling an outb message
    #[error("An error occurred handling an outb message {0:?}: {1}")]
    OutBHandlingError(String, String),

    /// Failed to get value from parameter value
    #[error("Failed To Convert Parameter Value {0:?} to {1:?}")]
    ParameterValueConversionFailure(ParameterValue, &'static str),

    /// a failure occured processing a PE file
    #[error("Failure processing PE File {0:?}")]
    PEFileProcessingFailure(#[from] goblin::error::Error),

    /// a Prometheus error occurred
    #[error("Prometheus Error {0:?}")]
    Prometheus(#[from] prometheus::Error),

    /// Raw pointer is less than base address
    #[error("Raw pointer ({0:?}) was less than the base address ({1})")]
    RawPointerLessThanBaseAddress(RawPtr, u64),

    /// RefCell borrow failed
    #[error("RefCell borrow failed")]
    RefCellBorrowFailed(#[from] BorrowError),

    /// RefCell mut borrow failed
    #[error("RefCell mut borrow failed")]
    RefCellMutBorrowFailed(#[from] BorrowMutError),

    /// Failed to get value from return value
    #[error("Failed To Convert Return Value {0:?} to {1:?}")]
    ReturnValueConversionFailure(ReturnValue, &'static str),

    /// Stack overflow detected in guest
    #[error("Stack overflow detected")]
    StackOverflow(),

    /// a backend error occurred with seccomp filters
    #[error("Backend Error with Seccomp Filter {0:?}")]
    #[cfg(all(feature = "seccomp", target_os = "linux"))]
    SeccompFilterBackendError(#[from] seccompiler::BackendError),

    /// an error occurred with seccomp filters
    #[error("Error with Seccomp Filter {0:?}")]
    #[cfg(all(feature = "seccomp", target_os = "linux"))]
    SeccompFilterError(#[from] seccompiler::Error),

    /// SystemTimeError
    #[error("SystemTimeError {0:?}")]
    SystemTimeError(#[from] SystemTimeError),

    /// Error occurred converting a slice to an array
    #[error("TryFromSliceError {0:?}")]
    TryFromSliceError(#[from] TryFromSliceError),

    /// The flatbuffer return value type is invalid
    #[error("The flatbuffer return value type is invalid {0:?}")]
    UnexpectedFlatBufferReturnValueType(FBReturnValue),

    /// A function was called with an incorrect number of arguments
    #[error("The number of arguments to the function is wrong: got {0:?} expected {1:?}")]
    UnexpectedNoOfArguments(usize, usize),

    /// The parameter value type is unexpected
    #[error("The parameter value type is unexpected got {0:?} expected {1:?}")]
    UnexpectedParameterValueType(ParameterValue, String),

    /// The return value type is unexpected
    #[error("The return value type is unexpected got {0:?} expected {1:?}")]
    UnexpectedReturnValueType(ReturnValue, String),

    /// The flatbuffer parameter type is invalid
    #[error("The flatbuffer parameter type is invalid {0:?}")]
    UnknownFlatBufferParameterType(ParameterType),

    /// The flatbuffer parameter value is invalid
    #[error("The flatbuffer parameter value is invalid {0:?}")]
    UnknownFlatBufferParameterValue(FBParameterValue),

    /// The flatbuffer return type is invalid
    #[error("The flatbuffer return type is invalid {0:?}")]
    UnknownFlatBufferReturnType(ReturnType),

    /// Slice conversion to UTF8 failed
    #[error("Slice Conversion of UTF8 data to str failed")]
    UTF8SliceConversionFailure(#[from] Utf8Error),

    /// Slice conversion to UTF8 failed
    #[error("String Conversion of UTF8 data to str failed")]
    UTF8StringConversionFailure(#[from] FromUtf8Error),

    /// The capacity of the vector is incorrect
    #[error(
        "The capacity of the vector is incorrect. Capacity: {0}, Length: {1}, FlatBuffer Size: {2}"
    )]
    VectorCapacityIncorrect(usize, usize, i32),

    /// vmm sys Error Occurred
    #[error("vmm sys Error {0:?}")]
    #[cfg(target_os = "linux")]
    VmmSysError(#[from] vmm_sys_util::errno::Error),

    /// Windows Error
    #[cfg(target_os = "windows")]
    #[error("Windows API Error Result {0:?}")]
    WindowsAPIError(#[from] windows::core::Error),

    /// Windows Error HRESULT
    #[cfg(target_os = "windows")]
    #[error("Windows API called returned an error HRESULT {0:?}")]
    WindowsErrorHResult(windows::core::HRESULT),
}

impl From<Infallible> for HyperlightError {
    fn from(_: Infallible) -> Self {
        "Impossible as this is an infallible error".into()
    }
}

impl From<&str> for HyperlightError {
    fn from(s: &str) -> Self {
        HyperlightError::Error(s.to_string())
    }
}

impl<T> From<PoisonError<MutexGuard<'_, T>>> for HyperlightError {
    // Implemented this way rather than passing the error as a source to LockAttemptFailed as that would require
    // Box<dyn Error + Send + Sync> which is not easy to implement for PoisonError<MutexGuard<'_, T>>
    // This is a good enough solution and allows use to use the ? operator on lock() calls
    fn from(e: PoisonError<MutexGuard<'_, T>>) -> Self {
        let source = match e.source() {
            Some(s) => s.to_string(),
            None => String::from(""),
        };
        HyperlightError::LockAttemptFailed(source)
    }
}

/// Creates a `HyperlightError::Error` from a string literal or format string
#[macro_export]
macro_rules! new_error {
    ($msg:literal $(,)?) => {{
        let __args = std::format_args!($msg);
        let __err_msg = match __args.as_str() {
            Some(msg) => String::from(msg),
            None => std::format!($msg),
        };
        $crate::HyperlightError::Error(__err_msg)
    }};
    ($fmtstr:expr, $($arg:tt)*) => {{
           let __err_msg = std::format!($fmtstr, $($arg)*);
           $crate::error::HyperlightError::Error(__err_msg)
    }};
}
