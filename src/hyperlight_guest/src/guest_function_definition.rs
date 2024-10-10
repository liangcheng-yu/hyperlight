use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{anyhow, Error, Result};
use hyperlight_common::flatbuffer_wrappers::function_types::{ParameterType, ReturnType};

/// The definition of a function exposed from the guest to the host
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestFunctionDefinition {
    /// The function name
    pub function_name: String,
    /// The type of the parameter values for the host function call.
    pub parameter_types: Vec<ParameterType>,
    /// The type of the return value from the host function call
    pub return_type: ReturnType,
    /// The function pointer to the guest function
    pub function_pointer: i64,
}

impl GuestFunctionDefinition {
    /// Create a new `GuestFunctionDefinition`.
    pub fn new(
        function_name: String,
        parameter_types: Vec<ParameterType>,
        return_type: ReturnType,
        function_pointer: i64,
    ) -> Self {
        Self {
            function_name,
            parameter_types,
            return_type,
            function_pointer,
        }
    }

    /// Verify equal parameter types
    pub fn verify_equal_parameter_types(
        &self,
        parameter_types: &[ParameterType],
    ) -> Result<(), Error> {
        if self.parameter_types.len() != parameter_types.len() {
            return Err(anyhow!(
                "Expected {} parameters, but got {}",
                self.parameter_types.len(),
                parameter_types.len()
            ));
        }

        for (i, parameter_type) in self.parameter_types.iter().enumerate() {
            if parameter_type != &parameter_types[i] {
                return Err(anyhow!("{}", i));
            }
        }

        Ok(())
    }

    /// Verify vector parameter lengths
    pub fn verify_vector_parameter_lengths(
        &self,
        parameter_types: Vec<ParameterType>,
    ) -> Result<(), Error> {
        // Check that:
        // - parameter_types doesn't end w/ a VecBytes parameter, and
        // - if parameter_types has a VecBytes parameter, then the next parameter is an integer
        //   specifying the length of that vector.
        let mut parameter_types_iter = parameter_types.iter();
        while let Some(parameter_type) = parameter_types_iter.next() {
            if parameter_type == &ParameterType::VecBytes {
                if let Some(ParameterType::Int) = parameter_types_iter.next() {
                    continue;
                } else {
                    return Err(anyhow!(
                        "Expected integer parameter after VecBytes parameter"
                    ));
                }
            }
        }

        Ok(())
    }
}
