use anyhow::{bail, Result};

use crate::guest::function_call::Param;

/// Wrapper for a vector of SupportedParameterOrReturnValue.
pub struct Parameters(pub Vec<SupportedParameterOrReturnValue>);

/// Type alias for a single SupportedParameterOrReturnValue.
pub type Return = SupportedParameterOrReturnValue;

/// All the value types that can be used as parameters or return types for a host function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportedParameterOrReturnValue {
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

impl From<Vec<SupportedParameterOrReturnValue>> for Parameters {
    fn from(value: Vec<SupportedParameterOrReturnValue>) -> Self {
        Parameters(value)
    }
}

impl TryFrom<Option<Vec<Param>>> for Parameters {
    type Error = anyhow::Error;

    fn try_from(value: Option<Vec<Param>>) -> Result<Self> {
        match value {
            Some(params) => params
                .into_iter()
                .map(|p| SupportedParameterOrReturnValue::try_from(p))
                .collect::<Result<Vec<SupportedParameterOrReturnValue>>>()
                .map(Parameters),
            None => Ok(Parameters(vec![])),
        }
    }
}

impl TryFrom<Param> for SupportedParameterOrReturnValue {
    type Error = anyhow::Error;

    fn try_from(value: Param) -> Result<Self> {
        match value {
            Param::Int(i) => Ok(SupportedParameterOrReturnValue::Int(i)),
            Param::Long(i) => Ok(SupportedParameterOrReturnValue::Long(i)),
            Param::Boolean(i) => Ok(SupportedParameterOrReturnValue::Bool(i)),
            Param::String(i) => Ok(SupportedParameterOrReturnValue::String(i.unwrap_or_default())),
            Param::VecBytes(i) => Ok(SupportedParameterOrReturnValue::ByteArray(i.unwrap_or_default())),
        }
    }
}

impl TryFrom<SupportedParameterOrReturnValue> for i32 {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterOrReturnValue) -> Result<Self> {
        match value {
            SupportedParameterOrReturnValue::Int(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i32", other),
        }
    }
}

impl TryFrom<SupportedParameterOrReturnValue> for i64 {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterOrReturnValue) -> Result<Self> {
        match value {
            SupportedParameterOrReturnValue::Long(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i64", other),
        }
    }
}

impl TryFrom<SupportedParameterOrReturnValue> for u64 {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterOrReturnValue) -> Result<Self> {
        match value {
            SupportedParameterOrReturnValue::ULong(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u64", other),
        }
    }
}

impl TryFrom<SupportedParameterOrReturnValue> for bool {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterOrReturnValue) -> Result<Self> {
        match value {
            SupportedParameterOrReturnValue::Bool(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to bool", other),
        }
    }
}

impl TryFrom<SupportedParameterOrReturnValue> for String {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterOrReturnValue) -> Result<Self> {
        match value {
            SupportedParameterOrReturnValue::String(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to String", other),
        }
    }
}

impl TryFrom<SupportedParameterOrReturnValue> for Vec<u8> {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterOrReturnValue) -> Result<Self> {
        match value {
            SupportedParameterOrReturnValue::ByteArray(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to Vec<u8>", other),
        }
    }
}

impl TryFrom<SupportedParameterOrReturnValue> for *mut std::ffi::c_void {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterOrReturnValue) -> Result<Self> {
        match value {
            SupportedParameterOrReturnValue::IntPtr(i) => Ok(i),
            other => bail!(
                "Invalid conversion: from {:?} to *mut std::ffi::c_void",
                other
            ),
        }
    }
}

impl TryFrom<SupportedParameterOrReturnValue> for u32 {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterOrReturnValue) -> Result<Self> {
        match value {
            SupportedParameterOrReturnValue::UInt(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u32", other),
        }
    }
}

impl TryFrom<SupportedParameterOrReturnValue> for () {
    type Error = anyhow::Error;

    fn try_from(value: SupportedParameterOrReturnValue) -> Result<Self> {
        match value {
            SupportedParameterOrReturnValue::Void(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to ()", other),
        }
    }
}
