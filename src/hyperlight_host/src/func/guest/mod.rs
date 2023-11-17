/// Represents a function call from host to guest.
pub mod function_call;
/// Represents the definition of a function that the guest exposes to the host.
pub(crate) mod function_definition;
/// Represents the functions that the guest exposes to the host.
pub(crate) mod function_details;
/// Represents guest log data
pub mod log_data;
/// An enumeration and supporting logic to determine the desired
/// level of a log message issued from the guest.
pub(crate) mod log_level;
