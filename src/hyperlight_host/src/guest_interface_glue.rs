use anyhow::{anyhow, Result};

use crate::guest::host_function_definition::HostFunctionDefinition;

type HostFunction = Box<
    dyn Fn(&[SupportedParameterAndReturnValues]) -> Result<SupportedParameterAndReturnValues>
        + Send
        + Sync,
>;

/// The definition of a function exposed from the host to the guest
pub struct HostMethodInfo {
    /// The function definition with the name, params, and return type
    pub host_function_definition: HostFunctionDefinition,
    /// The function pointer to the host function
    pub function_pointer: HostFunction,
}

/// All the types that can be used as parameters or return types for a host function.
pub enum SupportedParameterAndReturnTypes {
    /// i32
    Int,
    /// i64
    Long,
    /// u64
    ULong,
    /// bool
    Bool,
    /// StringF
    String,
    /// Vec<u8>
    ByteArray,
    /// *mut c_void (raw pointer to an unsized type)
    IntPtr,
    /// u32
    UInt,
    /// Void (return types only)
    Void,
}

/// All the value types that can be used as parameters or return types for a host function.
pub enum SupportedParameterAndReturnValues {
    /// i32
    Int(i32),
    /// i64
    Long(i64),
    /// u64
    ULong(u64),
    /// bool
    Bool(bool),
    /// String
    String(String),
    /// Vec<u8>
    ByteArray(Vec<u8>),
    /// *mut c_void (raw pointer to an unsized type)
    IntPtr(*mut std::ffi::c_void),
    /// u32
    UInt(u32),
    /// Void (return types only)
    Void(()),
}

/// Validates that the given type is supported by the host interface.
pub fn validate_type_supported(some_type: &str) -> Result<()> {
    // try to convert from &str to SupportedParameterAndReturnTypes
    match from_csharp_typename(some_type) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Converts from a C# type name to a SupportedParameterAndReturnTypes.
fn from_csharp_typename(value: &str) -> Result<SupportedParameterAndReturnTypes> {
    match value {
        "System.Int32" => Ok(SupportedParameterAndReturnTypes::Int),
        "System.Int64" => Ok(SupportedParameterAndReturnTypes::Long),
        "System.UInt64" => Ok(SupportedParameterAndReturnTypes::ULong),
        "System.Boolean" => Ok(SupportedParameterAndReturnTypes::Bool),
        "System.String" => Ok(SupportedParameterAndReturnTypes::String),
        "System.Byte[]" => Ok(SupportedParameterAndReturnTypes::ByteArray),
        "System.IntPtr" => Ok(SupportedParameterAndReturnTypes::IntPtr),
        "System.UInt32" => Ok(SupportedParameterAndReturnTypes::UInt),
        _ => Err(anyhow!("Unsupported type")),
    }
}
