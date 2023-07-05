use crate::guest_interface_glue::{
    SupportedParameterAndReturnTypes, SupportedParameterAndReturnValues,
};
use anyhow::{bail, Result};

/// This is a marker trait that is used to indicate that a type is a valid Hyperlight parameter type.
pub(crate) trait SupportedParameterType<T> {
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
            other => bail!(
                "Invalid conversion: from {:?} to *mut std::ffi::c_void",
                other
            ),
        }
    }
}
