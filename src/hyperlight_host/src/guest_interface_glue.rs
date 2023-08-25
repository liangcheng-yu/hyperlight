use anyhow::{bail, Result};

/// All the types that can be used as `Vec<ParameterValue>` or return types for a host
/// function.
pub enum SupportedParameterOrReturnType {
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

/// Validates that the given type is supported by the host interface.
pub fn validate_type_supported(some_type: &str) -> Result<()> {
    // try to convert from &str to SupportedParameterAndReturnTypes
    match from_csharp_typename(some_type) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Converts from a C# type name to a SupportedParameterAndReturnTypes.
fn from_csharp_typename(value: &str) -> Result<SupportedParameterOrReturnType> {
    match value {
        "System.Int32" => Ok(SupportedParameterOrReturnType::Int),
        "System.Int64" => Ok(SupportedParameterOrReturnType::Long),
        "System.UInt64" => Ok(SupportedParameterOrReturnType::ULong),
        "System.Boolean" => Ok(SupportedParameterOrReturnType::Bool),
        "System.String" => Ok(SupportedParameterOrReturnType::String),
        "System.Byte[]" => Ok(SupportedParameterOrReturnType::ByteArray),
        "System.IntPtr" => Ok(SupportedParameterOrReturnType::IntPtr),
        "System.UInt32" => Ok(SupportedParameterOrReturnType::UInt),
        other => bail!("Unsupported type: {:?}", other),
    }
}
