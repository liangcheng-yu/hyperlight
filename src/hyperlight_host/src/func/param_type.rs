use anyhow::{bail, Result};

use crate::func::types::{ParameterType, ParameterValue};

/// This is a marker trait that is used to indicate that a type is a
/// valid Hyperlight parameter type.
///
/// For each parameter type Hyperlight supports in host functions, we
/// provide an implementation for `SupporterParameterType<SupportedType>`
pub trait SupportedParameterType<T> {
    /// Get the underlying Hyperlight parameter type representing this
    /// `SupportedParameterType`
    fn get_hyperlight_type() -> ParameterType;
    /// Get the underling Hyperlight parameter value representing this
    /// `SupportedParameterType`
    fn get_hyperlight_value(&self) -> ParameterValue;
    /// Get the actual inner value of this `SupportedParameterType`
    fn get_inner(a: ParameterValue) -> Result<T>;
}

// We can then implement these traits for each type that Hyperlight supports as a parameter or return type
impl SupportedParameterType<String> for String {
    fn get_hyperlight_type() -> ParameterType {
        ParameterType::String
    }

    fn get_hyperlight_value(&self) -> ParameterValue {
        ParameterValue::String(self.clone())
    }

    fn get_inner(a: ParameterValue) -> Result<String> {
        match a {
            ParameterValue::String(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to String", other),
        }
    }
}

impl SupportedParameterType<i32> for i32 {
    fn get_hyperlight_type() -> ParameterType {
        ParameterType::Int
    }

    fn get_hyperlight_value(&self) -> ParameterValue {
        ParameterValue::Int(*self)
    }

    fn get_inner(a: ParameterValue) -> Result<i32> {
        match a {
            ParameterValue::Int(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i32", other),
        }
    }
}

impl SupportedParameterType<i64> for i64 {
    fn get_hyperlight_type() -> ParameterType {
        ParameterType::Long
    }

    fn get_hyperlight_value(&self) -> ParameterValue {
        ParameterValue::Long(*self)
    }

    fn get_inner(a: ParameterValue) -> Result<i64> {
        match a {
            ParameterValue::Long(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i64", other),
        }
    }
}

impl SupportedParameterType<bool> for bool {
    fn get_hyperlight_type() -> ParameterType {
        ParameterType::Bool
    }

    fn get_hyperlight_value(&self) -> ParameterValue {
        ParameterValue::Bool(*self)
    }

    fn get_inner(a: ParameterValue) -> Result<bool> {
        match a {
            ParameterValue::Bool(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to bool", other),
        }
    }
}

impl SupportedParameterType<Vec<u8>> for Vec<u8> {
    fn get_hyperlight_type() -> ParameterType {
        ParameterType::VecBytes
    }

    fn get_hyperlight_value(&self) -> ParameterValue {
        ParameterValue::VecBytes(self.clone())
    }

    fn get_inner(a: ParameterValue) -> Result<Vec<u8>> {
        match a {
            ParameterValue::VecBytes(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to Vec<u8>", other),
        }
    }
}
