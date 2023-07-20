/// Represents an error that occured int the guest.
pub(crate) mod error;
/// Represents a function call from host to guest.
pub(crate) mod function_call;
/// Represents the definition of a function that the guest exposes to the host.
pub(crate) mod function_definition;
/// Represents the functions that the guest exposes to the host.
pub(crate) mod function_details;
/// Represents guest log data
pub(crate) mod log_data;
/// An enumeration and supporting logic to determine the desired
/// level of a log message issued from the guest.
pub(crate) mod log_level;

use super::{ret_type::SupportedReturnType, types::ReturnValue};
use anyhow::Result;
use std::sync::{Arc, Mutex};

/// A Hyperlight function that takes no arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait GuestFunction<'a, R: SupportedReturnType<R>> {
    fn call(&self) -> Result<ReturnValue>;
}

impl<'a, T, R> GuestFunction<'a, R> for Arc<Mutex<T>>
where
    T: FnMut() -> anyhow::Result<R> + 'a + Send,
    R: SupportedReturnType<R>,
{
    fn call(&self) -> Result<ReturnValue> {
        let result = self
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?()?;
        Ok(result.get_hyperlight_value())
    }
}
