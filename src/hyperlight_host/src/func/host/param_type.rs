use anyhow::{bail, Result};

use super::{SupportedParameterOrReturnType, vals::SupportedParameterOrReturnValue};

/// This is a marker trait that is used to indicate that a type is a valid Hyperlight parameter type.
pub(crate) trait SupportedParameterType<T> {
    fn get_hyperlight_type() -> SupportedParameterOrReturnType;
    fn get_hyperlight_value(&self) -> SupportedParameterOrReturnValue;
    fn get_inner(a: SupportedParameterOrReturnValue) -> Result<T>;
}

// We can then implement these traits for each type that Hyperlight supports as a parameter or return type
impl SupportedParameterType<u32> for u32 {
    fn get_hyperlight_type() -> SupportedParameterOrReturnType {
        SupportedParameterOrReturnType::UInt
    }

    fn get_hyperlight_value(&self) -> SupportedParameterOrReturnValue {
        SupportedParameterOrReturnValue::UInt(*self)
    }

    fn get_inner(a: SupportedParameterOrReturnValue) -> Result<u32> {
        match a {
            SupportedParameterOrReturnValue::UInt(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u32", other),
        }
    }
}
impl SupportedParameterType<String> for String {
    fn get_hyperlight_type() -> SupportedParameterOrReturnType {
        SupportedParameterOrReturnType::String
    }

    fn get_hyperlight_value(&self) -> SupportedParameterOrReturnValue {
        SupportedParameterOrReturnValue::String(self.clone())
    }

    fn get_inner(a: SupportedParameterOrReturnValue) -> Result<String> {
        match a {
            SupportedParameterOrReturnValue::String(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to String", other),
        }
    }
}
impl SupportedParameterType<i32> for i32 {
    fn get_hyperlight_type() -> SupportedParameterOrReturnType {
        SupportedParameterOrReturnType::Int
    }

    fn get_hyperlight_value(&self) -> SupportedParameterOrReturnValue {
        SupportedParameterOrReturnValue::Int(*self)
    }

    fn get_inner(a: SupportedParameterOrReturnValue) -> Result<i32> {
        match a {
            SupportedParameterOrReturnValue::Int(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i32", other),
        }
    }
}
impl SupportedParameterType<i64> for i64 {
    fn get_hyperlight_type() -> SupportedParameterOrReturnType {
        SupportedParameterOrReturnType::Long
    }

    fn get_hyperlight_value(&self) -> SupportedParameterOrReturnValue {
        SupportedParameterOrReturnValue::Long(*self)
    }

    fn get_inner(a: SupportedParameterOrReturnValue) -> Result<i64> {
        match a {
            SupportedParameterOrReturnValue::Long(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to i64", other),
        }
    }
}
impl SupportedParameterType<u64> for u64 {
    fn get_hyperlight_type() -> SupportedParameterOrReturnType {
        SupportedParameterOrReturnType::ULong
    }

    fn get_hyperlight_value(&self) -> SupportedParameterOrReturnValue {
        SupportedParameterOrReturnValue::ULong(*self)
    }

    fn get_inner(a: SupportedParameterOrReturnValue) -> Result<u64> {
        match a {
            SupportedParameterOrReturnValue::ULong(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to u64", other),
        }
    }
}
impl SupportedParameterType<bool> for bool {
    fn get_hyperlight_type() -> SupportedParameterOrReturnType {
        SupportedParameterOrReturnType::Bool
    }

    fn get_hyperlight_value(&self) -> SupportedParameterOrReturnValue {
        SupportedParameterOrReturnValue::Bool(*self)
    }

    fn get_inner(a: SupportedParameterOrReturnValue) -> Result<bool> {
        match a {
            SupportedParameterOrReturnValue::Bool(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to bool", other),
        }
    }
}
impl SupportedParameterType<Vec<u8>> for Vec<u8> {
    fn get_hyperlight_type() -> SupportedParameterOrReturnType {
        SupportedParameterOrReturnType::ByteArray
    }

    fn get_hyperlight_value(&self) -> SupportedParameterOrReturnValue {
        SupportedParameterOrReturnValue::ByteArray(self.clone())
    }

    fn get_inner(a: SupportedParameterOrReturnValue) -> Result<Vec<u8>> {
        match a {
            SupportedParameterOrReturnValue::ByteArray(i) => Ok(i),
            other => bail!("Invalid conversion: from {:?} to Vec<u8>", other),
        }
    }
}
impl SupportedParameterType<*mut std::ffi::c_void> for *mut std::ffi::c_void {
    fn get_hyperlight_type() -> SupportedParameterOrReturnType {
        SupportedParameterOrReturnType::IntPtr
    }

    fn get_hyperlight_value(&self) -> SupportedParameterOrReturnValue {
        SupportedParameterOrReturnValue::IntPtr(*self)
    }

    fn get_inner(a: SupportedParameterOrReturnValue) -> Result<*mut std::ffi::c_void> {
        match a {
            SupportedParameterOrReturnValue::IntPtr(i) => Ok(i),
            other => bail!(
                "Invalid conversion: from {:?} to *mut std::ffi::c_void",
                other
            ),
        }
    }
}
