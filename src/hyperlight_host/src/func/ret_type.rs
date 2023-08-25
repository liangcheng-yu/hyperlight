use anyhow::{bail, Result};

use crate::func::types::{ReturnType, ReturnValue};

/// This is a marker trait that is used to indicate that a type is a valid Hyperlight return type.
pub trait SupportedReturnType<T> {
    /// Gets the return type of the supported return value
    fn get_hyperlight_type() -> ReturnType;

    /// Gets the value of the supported return value
    fn get_hyperlight_value(&self) -> ReturnValue;

    /// Gets the inner value of the supported return type
    fn get_inner(a: ReturnValue) -> Result<T>;
}

impl SupportedReturnType<()> for () {
    fn get_hyperlight_type() -> ReturnType {
        ReturnType::Void
    }

    fn get_hyperlight_value(&self) -> ReturnValue {
        ReturnValue::Void
    }

    fn get_inner(a: ReturnValue) -> Result<()> {
        match a {
            ReturnValue::Void => Ok(()),
            other => bail!("Invalid conversion: from {:?} to ()", other),
        }
    }
}

impl SupportedReturnType<String> for String {
    fn get_hyperlight_type() -> ReturnType {
        ReturnType::String
    }

    fn get_hyperlight_value(&self) -> ReturnValue {
        ReturnValue::String(self.clone())
    }

    fn get_inner(a: ReturnValue) -> Result<String> {
        match a {
            ReturnValue::String(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to String", other),
        }
    }
}

impl SupportedReturnType<i32> for i32 {
    fn get_hyperlight_type() -> ReturnType {
        ReturnType::Int
    }

    fn get_hyperlight_value(&self) -> ReturnValue {
        ReturnValue::Int(*self)
    }

    fn get_inner(a: ReturnValue) -> Result<i32> {
        match a {
            ReturnValue::Int(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i32", other),
        }
    }
}

impl SupportedReturnType<i64> for i64 {
    fn get_hyperlight_type() -> ReturnType {
        ReturnType::Long
    }

    fn get_hyperlight_value(&self) -> ReturnValue {
        ReturnValue::Long(*self)
    }

    fn get_inner(a: ReturnValue) -> Result<i64> {
        match a {
            ReturnValue::Long(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i64", other),
        }
    }
}

impl SupportedReturnType<bool> for bool {
    fn get_hyperlight_type() -> ReturnType {
        ReturnType::Bool
    }

    fn get_hyperlight_value(&self) -> ReturnValue {
        ReturnValue::Bool(*self)
    }

    fn get_inner(a: ReturnValue) -> Result<bool> {
        match a {
            ReturnValue::Bool(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to bool", other),
        }
    }
}

impl SupportedReturnType<Vec<u8>> for Vec<u8> {
    fn get_hyperlight_type() -> ReturnType {
        ReturnType::VecBytes
    }

    fn get_hyperlight_value(&self) -> ReturnValue {
        ReturnValue::VecBytes(self.clone())
    }

    fn get_inner(a: ReturnValue) -> Result<Vec<u8>> {
        match a {
            ReturnValue::VecBytes(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to Vec<u8>", other),
        }
    }
}
