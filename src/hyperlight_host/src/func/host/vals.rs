use anyhow::{bail, Result};

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
            other => bail!(
                "Invalid conversion: from {:?} to *mut std::ffi::c_void",
                other
            ),
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
