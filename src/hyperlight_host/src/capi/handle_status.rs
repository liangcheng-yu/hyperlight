use super::handle::Handle;
use super::hdl::Hdl;

/// The status of a `Handle`
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum HandleStatus {
    /// A valid `Handle` that is not an error or empty.
    ValidOther,
    /// A valid `Handle` that is empty
    ValidEmpty,
    /// A valid `Handle` that is an error type.
    /// You can call `handle_get_error_message` for this `Handle` and
    /// expect to get a string back that describes the error.
    ValidError,
    /// A `Handle` that is invalid. It is guaranteed to not reference any
    /// memory in any `Context`, nor can it be introspected by any Hyperlight
    /// C API functions
    Invalid,
}

/// Return the status of a `Handle`
#[no_mangle]
pub extern "C" fn handle_get_status(hdl: Handle) -> HandleStatus {
    match Hdl::try_from(hdl) {
        Err(_) => HandleStatus::Invalid,
        Ok(Hdl::Err(_)) => HandleStatus::ValidError,
        Ok(Hdl::Empty()) => HandleStatus::ValidEmpty,
        _ => HandleStatus::ValidOther,
    }
}
