use crate::Result;
/// Definitions for common functions to be exposed in the guest
pub mod exports;
/// Represents a function call.
pub mod function_call;
/// Types used to pass data to/from the guest.
pub mod guest;
/// Definitions and functionality to enable guest-to-host function calling,
/// also called "host functions"
///
/// This module includes functionality to do the following
///
/// - Define several prototypes for what a host function must look like,
/// including the number of arguments (arity) they can have, supported argument
/// types, and supported return types
/// - Registering host functions to be callable by the guest
/// - Dynamically dispatching a call from the guest to the appropriate
/// host function
pub mod host;
/// Definitions and functionality for supported parameter types
pub(crate) mod param_type;
/// Definitions and functionality for supported return types
pub mod ret_type;
/// Definitions for types related to functions used by both the guest and the
/// host. This includes the types of parameters and return values that are
/// supported in Hyperlight.
pub mod types;

pub use param_type::SupportedParameterType;
pub use ret_type::SupportedReturnType;
use std::sync::{Arc, Mutex};
pub use types::ParameterValue;
pub use types::ReturnType;
pub use types::ReturnValue;

type HLFunc<'a> =
    Arc<Mutex<Box<dyn FnMut(Vec<ParameterValue>) -> Result<ReturnValue> + 'a + Send>>>;

/// Generic HyperlightFunction
#[derive(Clone)]
pub struct HyperlightFunction<'a>(HLFunc<'a>);

impl<'a> HyperlightFunction<'a> {
    pub(crate) fn new<F>(f: F) -> Self
    where
        F: FnMut(Vec<ParameterValue>) -> Result<ReturnValue> + 'a + Send,
    {
        Self(Arc::new(Mutex::new(Box::new(f))))
    }

    pub(crate) fn call(&self, args: Vec<ParameterValue>) -> Result<ReturnValue> {
        let mut f = self.0.lock().unwrap();
        f(args)
    }
}

/// Re-export for `get_stack_boundary` function
pub use exports::get_stack_boundary;
/// Re-export for `HostFunction0` trait
pub use host::HostFunction0;
/// Re-export for `HostFunction1` trait
pub use host::HostFunction1;
