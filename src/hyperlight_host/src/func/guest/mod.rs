/// Represents an error that occured int the guest.
pub mod error;
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

use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::Sandbox;

/// A simple guest function that takes no arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub trait GuestFunction<R> {
    /// Call the guest function
    fn call(&self, s: Arc<Mutex<&mut Sandbox>>) -> Result<R>;
}

impl<'a, T, R> GuestFunction<R> for Arc<Mutex<T>>
where
    T: FnMut(Arc<Mutex<&mut Sandbox>>) -> anyhow::Result<R> + 'a + Send,
{
    fn call(&self, s: Arc<Mutex<&mut Sandbox>>) -> Result<R> {
        let result = self
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(s)?;
        Ok(result)
    }
}
