use readonly;
/// The definition of a function exposed from the host to the guest
#[readonly::make]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HostFunctionDefinition {
    /// The function name
    pub function_name: String,
    /// The type of the parameter values for the host function call.
    pub parameter_types: Option<Vec<ParamValueType>>,
    /// The type of the return value from the host function call
    pub return_type: ReturnValueType,
}

/// This is the type of a parameter that can be passed to a host function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamValueType {
    /// Parameter is a signed 32 bit integer.
    Int,
    /// Parameter is a signed 64 bit integer.
    Long,
    /// Parameter is a boolean.
    Boolean,
    /// Parameter is a string.
    String,
    /// Parameter is a vector of bytes.
    VecBytes,
}

/// This is the type of a value that can be returned from a host function.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ReturnValueType {
    #[default]
    /// Return value is a signed 32 bit integer.
    Int,
    /// Return value is a signed 64 bit integer.
    Long,
    /// Return value is a boolean.
    Boolean,
    /// Return value is a string.
    String,
    /// Return value is void.
    Void,
}

impl HostFunctionDefinition {
    /// Create a new `HostFunctionDetails`.
    pub fn new(
        function_name: String,
        parameter_types: Option<Vec<ParamValueType>>,
        return_type: ReturnValueType,
    ) -> Self {
        Self {
            function_name,
            parameter_types,
            return_type,
        }
    }
}
