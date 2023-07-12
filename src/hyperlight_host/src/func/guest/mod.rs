/// Represents an error that occured int the guest.
pub mod error;

/// Represents a function call from host to guest.
pub mod function_call;

/// Represents the definition of a function that the guest exposes to the host.
pub mod function_definition;

/// Represents the functions that the guest exposes to the host.
pub mod function_details;

/// Represents guest log data
pub mod log_data;

/// An enumeration and supporting logic to determine the desired
/// level of a log message issued from the guest.
pub mod log_level;
