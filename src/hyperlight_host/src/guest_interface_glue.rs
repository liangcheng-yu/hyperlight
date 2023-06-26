use anyhow::{anyhow, Result};

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

impl SupportedParameterAndReturnValues {
    /// Gets the inner value as a reference to a dyn Any
    pub fn get_inner(&self) -> Result<&dyn std::any::Any> {
        match self {
            SupportedParameterAndReturnValues::Int(x) => Ok(x),
            SupportedParameterAndReturnValues::Long(x) => Ok(x),
            SupportedParameterAndReturnValues::ULong(x) => Ok(x),
            SupportedParameterAndReturnValues::Bool(x) => Ok(x),
            SupportedParameterAndReturnValues::String(x) => Ok(x),
            SupportedParameterAndReturnValues::ByteArray(x) => Ok(x),
            SupportedParameterAndReturnValues::IntPtr(x) => Ok(x),
            SupportedParameterAndReturnValues::UInt(x) => Ok(x),
            SupportedParameterAndReturnValues::Void(x) => Ok(x),
        }
    }
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

/// This is a marker trait that is used to indicate that a type is a valid Hyperlight parameter type.
pub(crate) trait SupportedParameterType {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes;
    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues;
}
/// This is a marker trait that is used to indicate that a type is a valid Hyperlight return type.
pub(crate) trait SupportedReturnType {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes;
    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues;
}

// We can then implement these traits for each type that Hyperlight supports as a parameter or return type
impl SupportedParameterType for u32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::UInt
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::UInt(*self)
    }
}
impl SupportedParameterType for String {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::String
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::String(self.clone())
    }
}
impl SupportedParameterType for i32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Int
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Int(*self)
    }
}
impl SupportedParameterType for i64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Long
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Long(*self)
    }
}
impl SupportedParameterType for u64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ULong
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::ULong(*self)
    }
}
impl SupportedParameterType for bool {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Bool
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Bool(*self)
    }
}
impl SupportedParameterType for Vec<u8> {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ByteArray
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::ByteArray(self.clone())
    }
}
impl SupportedParameterType for *mut std::ffi::c_void {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::IntPtr
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::IntPtr(*self)
    }
}

impl SupportedReturnType for u32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::UInt
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::UInt(*self)
    }
}
impl SupportedReturnType for () {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Void
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Void(())
    }
}
impl SupportedReturnType for String {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::String
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::String(self.clone())
    }
}
impl SupportedReturnType for i32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Int
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Int(*self)
    }
}
impl SupportedReturnType for i64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Long
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Long(*self)
    }
}
impl SupportedReturnType for u64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ULong
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::ULong(*self)
    }
}
impl SupportedReturnType for bool {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Bool
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Bool(*self)
    }
}
impl SupportedReturnType for Vec<u8> {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ByteArray
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::ByteArray(self.clone())
    }
}
impl SupportedReturnType for *mut std::ffi::c_void {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::IntPtr
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::IntPtr(*self)
    }
}
