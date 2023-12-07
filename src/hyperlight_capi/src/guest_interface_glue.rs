use super::{c_func::CFunc, context::Context, handle::Handle};
use crate::hdl::Hdl;
use hyperlight_host::{new_error, Result};
/// Validates that the given type is supported by the host interface.
///
/// # Safety
/// - This function expects to receive a Handle to a String as the second argument.
/// - If type is not supported, it will yield an error Handle.
#[no_mangle]
pub unsafe extern "C" fn guest_interface_glue_validate_type_supported(
    context: *mut Context,
    some_type: Handle,
) -> Handle {
    CFunc::new("guest_interface_glue_validate_type_supported", context)
        .and_then_mut(|ctx, _| {
            let some_type = Context::get(some_type, &ctx.strings, |h| matches!(h, Hdl::String(_)))?;
            validate_type_supported(some_type).map(|_| Handle::new_empty())
        })
        .ok_or_err_hdl()
}

/// All the types that can be used as `Vec<ParameterValue>` or return types for a host
/// function.
enum SupportedParameterOrReturnType {
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
fn validate_type_supported(some_type: &str) -> Result<()> {
    // try to convert from &str to SupportedParameterAndReturnTypes
    from_csharp_typename(some_type).map(|_| ())
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
        "System.Void" => Ok(SupportedParameterOrReturnType::Void),
        other => Err(new_error!("Unsupported type: {}", other)),
    }
}
