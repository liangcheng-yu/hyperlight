use std::{
    array::TryFromSliceError,
    cell::BorrowError,
    cell::BorrowMutError,
    convert::Infallible,
    error::Error,
    num::TryFromIntError,
    str::Utf8Error,
    string::FromUtf8Error,
    sync::{MutexGuard, PoisonError},
    time::SystemTimeError,
};

use hyperlight_flatbuffers::{
    flatbuffer_wrappers::guest_error::ErrorCode,
    flatbuffers::hyperlight::generated::{
        FunctionCallType, ParameterType, ParameterValue as FBParameterValue, ReturnType,
        ReturnValue as FBReturnValue,
    },
};

use crate::mem::ptr::RawPtr;
#[cfg(target_os = "windows")]
use crossbeam_channel::{RecvError, SendError};
use flatbuffers::InvalidFlatbuffer;
use hyperlight_flatbuffers::flatbuffer_wrappers::function_types::{ParameterValue, ReturnValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub(crate) struct HyperlightHostError {
    pub(crate) message: String,
    pub(crate) source: String,
}

#[derive(Error, Debug)]
/// The error type for Hyperlight operations
pub enum HyperlightError {
    #[error("Offset: {0} out of bounds, Max is: {1}")]
    /// Memory access out of bounds
    BoundsCheckFailed(u64, usize),
    #[error("Call_entry_point is only available with in-process mode")]
    ///Call entry point was callled when not in process
    CallEntryPointIsInProcOnly(),
    #[error("Couldnt add offset to base address. Offset: {0}, Base Address: {1}")]
    /// Checked Add Overflow
    CheckedAddOverflow(u64, u64),
    #[error("{0:?}")]
    #[cfg(target_os = "windows")]
    /// Cross beam channel receive error
    CrossBeamReceiveError(#[from] RecvError),
    #[error("{0:?}")]
    #[cfg(target_os = "windows")]
    /// Cross beam channel send error
    CrossBeamSendError(#[from] SendError<HANDLE>),
    #[error("Error converting CString {0:?}")]
    /// CString conversion error
    CStringConversionError(#[from] std::ffi::NulError),
    #[error("{0}")]
    /// A generic error with a message
    Error(String),
    #[error("Exception Data Length is incorrect. Expected: {0}, Actual: {1}")]
    /// Exception Data Length is incorrect
    ExceptionDataLengthIncorrect(i32, usize),
    #[error("Exception Message is too big. Max Size: {0}, Actual: {1}")]
    /// Exception Message is too big
    ExceptionMessageTooBig(usize, usize),
    #[error("Execution was cancelled by the host.")]
    /// Guest execution was cancelled by the host
    ExecutionCanceledByHost(),
    #[error("Failed to get a value from flat buffer parameter")]
    /// Accessing the value of a flatbuffer parameter failed
    FailedToGetValueFromParameter(),
    #[error("Field Name {0} not found in decoded GuestLogData")]
    ///Field Name not found in decoded GuestLogData
    FieldIsMissingInGuestLogData(String),
    #[error("Cannot run from guest binary when guest binary is a buffer")]
    ///Cannot run from guest binary unless the binary is a file
    GuestBinaryShouldBeAFile(),
    #[error("Guest aborted: {0} {1}")]
    /// Guest aborted during outb
    GuestAborted(u8, String),
    #[error("Guest error occurred {0:?}: {1}")]
    /// Guest call resulted in error in guest
    GuestError(ErrorCode, String),
    #[error("Guest call is already in progress")]
    /// Guest call already in progress
    GuestFunctionCallAlreadyInProgress(),
    #[error("Unsupported type: {0}")]
    /// The given type is not supported by the guest interface.
    GuestInterfaceUnsupportedType(String),
    #[error("The guest offset {0} is invalid.")]
    /// The guest offset is invalid.
    GuestOffsetIsInvalid(usize),
    #[error("Failed to cancel guest execution.")]
    /// An attempt to cancel guest execution failed
    HostFailedToCancelGuestExecution(),
    #[error("Guest executon was cancelled but the guest did not exit after sending {0} signals")]
    #[cfg(target_os = "linux")]
    /// Guest executon was cancelled but the guest did not exit after sending signals
    HostFailedToCancelGuestExecutionSendingSignals(i32),
    #[error("HostFunction {0} was not found")]
    /// A Host function was called by the guest but it was not registered.
    HostFunctionNotFound(String),
    #[error("Failed To Convert Size to usize")]
    /// Failed to convert to Integer
    IntConversionFailure(#[from] TryFromIntError),
    #[error("The flatbuffer is invalid")]
    /// The flatbuffer is invalid
    InvalidFlatBuffer(#[from] InvalidFlatbuffer),
    #[error("The function call type is invalid {0:?}")]
    /// The function call type is invalid
    InvalidFunctionCallType(FunctionCallType),
    #[error("Reading Writing or Seeking data failed {0:?}")]
    /// Reading Writing or Seeking data failed.
    IOError(#[from] std::io::Error),
    #[error("Conversion of str data to json failed")]
    /// Conversion of str to Json failed
    JsonConversionFailure(#[source] serde_json::Error),
    #[error("Unable to lock resource")]
    /// An attempt to get a lock from a Mutex failed.
    LockAttemptFailed(String),
    #[error("KVM Error {0:?}")]
    #[cfg(target_os = "linux")]
    /// Error occurred in KVM Operation
    KVMError(#[from] kvm_ioctls::Error),
    #[error("Memory Allocation Failed with OS Error {0:?}.")]
    /// Memory Allocation Failed.
    MemoryAllocationFailed(Option<i32>),
    #[error("Memory requested {0} exceeds maximum size allowed {1}")]
    /// The memory request exceeds the maximum size allowed
    MemoryRequestTooBig(usize, usize),
    #[error("Metric Not Found {0:?}.")]
    /// Metric Not Found.
    MetricNotFound(&'static str),
    #[error("mmap failed with os error {0:?}")]
    /// mmap Failed.
    MmapFailed(Option<i32>),
    #[error("mshv Error {0:?}")]
    #[cfg(target_os = "linux")]
    /// mshv Error Occurred
    MSHVError(#[from] vmm_sys_util::errno::Error),
    #[error("No Hypervisor was found for Sandbox")]
    /// No Hypervisor was found for Sandbox.
    NoHypervisorFound(),
    #[error("Restore_state called with no valid snapshot")]
    /// Restore_state called with no valid snapshot
    NoMemorySnapshot,
    #[error("An error occurred handling an outb message {0:?}: {1}")]
    /// An error occurred handling an outb message
    OutBHandlingError(String, String),
    #[error("Failed To Convert Parameter Value {0:?} to {1:?}")]
    /// Failed to get value from parameter value
    ParameterValueConversionFailure(ParameterValue, &'static str),
    #[error("Failure processing PE File {0:?}")]
    /// a failure occured processing a PE file
    PEFileProcessingFailure(#[from] goblin::error::Error),
    #[error("Prometheus Error {0:?}")]
    /// a Prometheus error occurred
    Prometheus(#[from] prometheus::Error),
    #[error("Raw pointer ({0:?}) was less than the base address ({1})")]
    /// Raw pointer is less than base address
    RawPointerLessThanBaseAddress(RawPtr, u64),
    #[error("RefCell borrow failed")]
    /// RefCell borrow failed
    RefCellBorrowFailed(#[from] BorrowError),
    #[error("RefCell mut borrow failed")]
    /// RefCell mut borrow failed
    RefCellMutBorrowFailed(#[from] BorrowMutError),
    #[error("Failed To Convert Return Value {0:?} to {1:?}")]
    /// Failed to get value from return value
    ReturnValueConversionFailure(ReturnValue, &'static str),
    #[error("Stack overflow detected")]
    /// Stack overflow detected in guest
    StackOverflow(),
    #[error("SystemTimeError {0:?}")]
    /// SystemTimeError
    SystemTimeError(#[from] SystemTimeError),
    #[error("TryFromSliceError {0:?}")]
    /// Error occurred converting a slice to an array
    TryFromSliceError(#[from] TryFromSliceError),
    #[error("The flatbuffer return value type is invalid {0:?}")]
    /// The flatbuffer return value type is invalid
    UnexpectedFlatBufferReturnValueType(FBReturnValue),
    #[error("The number of arguments to the function is wrong: got {0:?} expected {1:?}")]
    /// A function was called with an incorrect number of arguments
    UnexpectedNoOfArguments(usize, usize),
    #[error("The parameter value type is unexpected got {0:?} expected {1:?}")]
    /// The parameter value type is unexpected
    UnexpectedParameterValueType(ParameterValue, String),
    #[error("The return value type is unexpected got {0:?} expected {1:?}")]
    /// The return value type is unexpected
    UnexpectedReturnValueType(ReturnValue, String),
    #[error("The flatbuffer parameter type is invalid {0:?}")]
    /// The flatbuffer parameter type is invalid
    UnknownFlatBufferParameterType(ParameterType),
    #[error("The flatbuffer parameter value is invalid {0:?}")]
    /// The flatbuffer parameter value is invalid
    UnknownFlatBufferParameterValue(FBParameterValue),
    #[error("The flatbuffer return type is invalid {0:?}")]
    /// The flatbuffer return type is invalid
    UnknownFlatBufferReturnType(ReturnType),
    #[error("Slice Conversion of UTF8 data to str failed")]
    /// Slice conversion to UTF8 failed
    UTF8SliceConversionFailure(#[from] Utf8Error),
    #[error("String Conversion of UTF8 data to str failed")]
    /// Slice conversion to UTF8 failed
    UTF8StringConversionFailure(#[from] FromUtf8Error),
    #[error(
        "The capacity of the vector is incorrect. Capacity: {0}, Length: {1}, FlatBuffer Size: {2}"
    )]
    /// The capacity of the vector is is incorrect
    VectorCapacityInCorrect(usize, usize, i32),
    #[error("Windows API Error Result {0:?}")]
    #[cfg(target_os = "windows")]
    /// Windows Error
    WindowsAPIError(#[from] windows::core::Error),
    #[error("Windows API called returned an error HRESULT {0:?}")]
    #[cfg(target_os = "windows")]
    /// Windows Error HRESULT
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
