use anyhow::{bail, Result};

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
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl TryFrom<SupportedParameterAndReturnValues> for i32 {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnValues) -> Result<Self> {
        match value {
            SupportedParameterAndReturnValues::Int(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i32", other),
        }
    }
}

impl TryFrom<SupportedParameterAndReturnValues> for i64 {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnValues) -> Result<Self> {
        match value {
            SupportedParameterAndReturnValues::Long(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i64", other),
        }
    }
}

impl TryFrom<SupportedParameterAndReturnValues> for u64 {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnValues) -> Result<Self> {
        match value {
            SupportedParameterAndReturnValues::ULong(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u64", other),
        }
    }
}

impl TryFrom<SupportedParameterAndReturnValues> for bool {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnValues) -> Result<Self> {
        match value {
            SupportedParameterAndReturnValues::Bool(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to bool", other),
        }
    }
}

impl TryFrom<SupportedParameterAndReturnValues> for String {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnValues) -> Result<Self> {
        match value {
            SupportedParameterAndReturnValues::String(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to String", other),
        }
    }
}

impl TryFrom<SupportedParameterAndReturnValues> for Vec<u8> {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnValues) -> Result<Self> {
        match value {
            SupportedParameterAndReturnValues::ByteArray(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to Vec<u8>", other),
        }
    }
}

impl TryFrom<SupportedParameterAndReturnValues> for *mut std::ffi::c_void {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnValues) -> Result<Self> {
        match value {
            SupportedParameterAndReturnValues::IntPtr(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to *mut std::ffi::c_void", other),
        }
    }
}

impl TryFrom<SupportedParameterAndReturnValues> for u32 {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnValues) -> Result<Self> {
        match value {
            SupportedParameterAndReturnValues::UInt(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u32", other),
        }
    }
}

impl TryFrom<SupportedParameterAndReturnValues> for () {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnValues) -> Result<Self> {
        match value {
            SupportedParameterAndReturnValues::Void(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to ()", other),
        }
    }
}

impl TryFrom<SupportedParameterAndReturnTypes> for u32 {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterAndReturnTypes) -> Result<Self> {
        match value {
            SupportedParameterAndReturnTypes::Int => Ok(0),
            other => bail!("Invalid conversion: from {:?} to u32", other),
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
        other => bail!("Unsupported type: {:?}", other),
    }
}

/// This is a marker trait that is used to indicate that a type is a valid Hyperlight parameter type.
pub(crate) trait SupportedParameterType<T> {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes;
    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues;
    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<T>;
}
/// This is a marker trait that is used to indicate that a type is a valid Hyperlight return type.
pub(crate) trait SupportedReturnType<T> {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes;
    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues;
    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<T>;
}

// We can then implement these traits for each type that Hyperlight supports as a parameter or return type
impl SupportedParameterType<u32> for u32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::UInt
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::UInt(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<u32> {
        match a {
            SupportedParameterAndReturnValues::UInt(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u32", other),
        }
    }
}
impl SupportedParameterType<String> for String {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::String
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::String(self.clone())
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<String> {
        match a {
            SupportedParameterAndReturnValues::String(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to String", other),
        }
    }
}
impl SupportedParameterType<i32> for i32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Int
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Int(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<i32> {
        match a {
            SupportedParameterAndReturnValues::Int(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i32", other),
        }
    }
}
impl SupportedParameterType<i64> for i64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Long
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Long(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<i64> {
        match a {
            SupportedParameterAndReturnValues::Long(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i64", other),
        }
    }
}
impl SupportedParameterType<u64> for u64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ULong
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::ULong(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<u64> {
        match a {
            SupportedParameterAndReturnValues::ULong(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u64", other),
        }
    }
}
impl SupportedParameterType<bool> for bool {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Bool
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Bool(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<bool> {
        match a {
            SupportedParameterAndReturnValues::Bool(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to bool", other),
        }
    }
}
impl SupportedParameterType<Vec<u8>> for Vec<u8> {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ByteArray
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::ByteArray(self.clone())
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<Vec<u8>> {
        match a {
            SupportedParameterAndReturnValues::ByteArray(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to Vec<u8>", other),
        }
    }
}
impl SupportedParameterType<*mut std::ffi::c_void> for *mut std::ffi::c_void {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::IntPtr
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::IntPtr(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<*mut std::ffi::c_void> {
        match a {
            SupportedParameterAndReturnValues::IntPtr(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to *mut std::ffi::c_void", other),
        }
    }
}

impl SupportedReturnType<u32> for u32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::UInt
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::UInt(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<u32> {
        match a {
            SupportedParameterAndReturnValues::UInt(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u32", other),
        }
    }
}
impl SupportedReturnType<()> for () {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Void
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Void(())
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<()> {
        match a {
            SupportedParameterAndReturnValues::Void(_) => Ok(()),
            other => bail!("Invalid conversion: from {:?} to ()", other),
        }
    }
}
impl SupportedReturnType<String> for String {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::String
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::String(self.clone())
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<String> {
        match a {
            SupportedParameterAndReturnValues::String(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to String", other),
        }
    }
}
impl SupportedReturnType<i32> for i32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Int
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Int(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<i32> {
        match a {
            SupportedParameterAndReturnValues::Int(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i32", other),
        }
    }
}
impl SupportedReturnType<i64> for i64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Long
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Long(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<i64> {
        match a {
            SupportedParameterAndReturnValues::Long(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i64", other),
        }
    }
}
impl SupportedReturnType<u64> for u64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ULong
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::ULong(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<u64> {
        match a {
            SupportedParameterAndReturnValues::ULong(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u64", other),
        }
    }
}
impl SupportedReturnType<bool> for bool {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Bool
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::Bool(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<bool> {
        match a {
            SupportedParameterAndReturnValues::Bool(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to bool", other),
        }
    }
}
impl SupportedReturnType<Vec<u8>> for Vec<u8> {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ByteArray
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::ByteArray(self.clone())
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<Vec<u8>> {
        match a {
            SupportedParameterAndReturnValues::ByteArray(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to Vec<u8>", other),
        }
    }
}
impl SupportedReturnType<*mut std::ffi::c_void> for *mut std::ffi::c_void {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::IntPtr
    }

    fn get_hyperlight_value(&self) -> SupportedParameterAndReturnValues {
        SupportedParameterAndReturnValues::IntPtr(*self)
    }

    fn get_inner(a: SupportedParameterAndReturnValues) -> Result<*mut std::ffi::c_void> {
        match a {
            SupportedParameterAndReturnValues::IntPtr(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to *mut std::ffi::c_void", other),
        }
    }
}
