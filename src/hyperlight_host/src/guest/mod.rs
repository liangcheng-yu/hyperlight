///! Represents a function call.
pub mod function_call;

///! Represents an error that occured int the guest.
pub mod guest_error;

///! Represents a function call from host to guest.
pub mod guest_function_call;

///! Represents the functions that the host exposes to the guest.
pub mod host_function_details;

///! Represents the definition of a function that the host exposes to the guest.
pub mod host_function_definition;

///! Represents a function call from guest to host.
pub mod host_function_call;
