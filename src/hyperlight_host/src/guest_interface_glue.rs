use anyhow::{anyhow, Result};

/// All the types that can be used as parameters or return types for a host function.
enum SupportedParameterAndReturnTypes {
    Int,
    Long,
    ULong,
    Bool,
    String,
    ByteArray,
    IntPtr,
    UInt32,
}

/// DAN:TODO
pub fn validate_type_supported(some_type: &str) -> Result<()> {
    // try to convert from &str to SupportedParameterAndReturnTypes
    match SupportedParameterAndReturnTypes::try_from(some_type) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

impl TryFrom<&str> for SupportedParameterAndReturnTypes {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "System.Int32" => Ok(SupportedParameterAndReturnTypes::Int),
            "System.Int64" => Ok(SupportedParameterAndReturnTypes::Long),
            "System.UInt64" => Ok(SupportedParameterAndReturnTypes::ULong),
            "System.Boolean" => Ok(SupportedParameterAndReturnTypes::Bool),
            "System.String" => Ok(SupportedParameterAndReturnTypes::String),
            "System.Byte[]" => Ok(SupportedParameterAndReturnTypes::ByteArray),
            "System.IntPtr" => Ok(SupportedParameterAndReturnTypes::IntPtr),
            "System.UInt32" => Ok(SupportedParameterAndReturnTypes::UInt32),
            _ => Err(anyhow!("Unsupported type")),
        }
    }
}
