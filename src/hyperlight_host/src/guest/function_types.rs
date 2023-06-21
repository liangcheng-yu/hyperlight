use crate::flatbuffers::hyperlight::generated::{
    ParameterType as FbParameterType, ReturnType as FbReturnType,
};
use anyhow::{bail, Result};

/// This is the type of a parameter that can be passed to a host function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamValueType {
    /// Parameter is a signed 32 bit integer.
    Int,
    /// Parameter is a signed 64 bit integer.
    Long,
    /// Parameter is a boolean.
    Boolean,
    /// Parameter is a string.
    String,
    /// Parameter is a vector of bytes.
    VecBytes,
}

/// This is the type of a value that can be returned from a host function.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ReturnValueType {
    #[default]
    /// Return value is a signed 32 bit integer.
    Int,
    /// Return value is a signed 64 bit integer.
    Long,
    /// Return value is a boolean.
    Boolean,
    /// Return value is a string.
    String,
    /// Return value is void.
    Void,
}

impl From<ReturnValueType> for FbReturnType {
    fn from(value: ReturnValueType) -> Self {
        match value {
            ReturnValueType::Int => FbReturnType::hlint,
            ReturnValueType::Long => FbReturnType::hllong,
            ReturnValueType::String => FbReturnType::hlstring,
            ReturnValueType::Boolean => FbReturnType::hlbool,
            ReturnValueType::Void => FbReturnType::hlvoid,
        }
    }
}

impl TryFrom<FbReturnType> for ReturnValueType {
    type Error = anyhow::Error;
    fn try_from(value: FbReturnType) -> Result<Self> {
        match value {
            FbReturnType::hlint => Ok(ReturnValueType::Int),
            FbReturnType::hllong => Ok(ReturnValueType::Long),
            FbReturnType::hlstring => Ok(ReturnValueType::String),
            FbReturnType::hlbool => Ok(ReturnValueType::Boolean),
            FbReturnType::hlvoid => Ok(ReturnValueType::Void),
            _ => bail!("Unknown return type: {:?}", value),
        }
    }
}

impl TryFrom<FbParameterType> for ParamValueType {
    type Error = anyhow::Error;
    fn try_from(value: FbParameterType) -> Result<Self> {
        match value {
            FbParameterType::hlint => Ok(ParamValueType::Int),
            FbParameterType::hllong => Ok(ParamValueType::Long),
            FbParameterType::hlstring => Ok(ParamValueType::String),
            FbParameterType::hlbool => Ok(ParamValueType::Boolean),
            FbParameterType::hlvecbytes => Ok(ParamValueType::VecBytes),
            _ => bail!("Unknown parameter type: {:?}", value),
        }
    }
}

impl From<ParamValueType> for FbParameterType {
    fn from(value: ParamValueType) -> Self {
        match value {
            ParamValueType::Int => FbParameterType::hlint,
            ParamValueType::Long => FbParameterType::hllong,
            ParamValueType::String => FbParameterType::hlstring,
            ParamValueType::Boolean => FbParameterType::hlbool,
            ParamValueType::VecBytes => FbParameterType::hlvecbytes,
        }
    }
}
