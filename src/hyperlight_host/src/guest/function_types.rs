use crate::flatbuffers::hyperlight::generated::{
    ParameterType as FbParameterType, ReturnType as FbReturnType,
};
use anyhow::{bail, Result};

/// This is the type of a parameter that can be passed to a host function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamType {
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
pub enum ReturnType {
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

impl From<ReturnType> for FbReturnType {
    fn from(value: ReturnType) -> Self {
        match value {
            ReturnType::Int => FbReturnType::hlint,
            ReturnType::Long => FbReturnType::hllong,
            ReturnType::String => FbReturnType::hlstring,
            ReturnType::Boolean => FbReturnType::hlbool,
            ReturnType::Void => FbReturnType::hlvoid,
        }
    }
}

impl TryFrom<FbReturnType> for ReturnType {
    type Error = anyhow::Error;
    fn try_from(value: FbReturnType) -> Result<Self> {
        match value {
            FbReturnType::hlint => Ok(ReturnType::Int),
            FbReturnType::hllong => Ok(ReturnType::Long),
            FbReturnType::hlstring => Ok(ReturnType::String),
            FbReturnType::hlbool => Ok(ReturnType::Boolean),
            FbReturnType::hlvoid => Ok(ReturnType::Void),
            _ => bail!("Unknown return type: {:?}", value),
        }
    }
}

impl TryFrom<FbParameterType> for ParamType {
    type Error = anyhow::Error;
    fn try_from(value: FbParameterType) -> Result<Self> {
        match value {
            FbParameterType::hlint => Ok(ParamType::Int),
            FbParameterType::hllong => Ok(ParamType::Long),
            FbParameterType::hlstring => Ok(ParamType::String),
            FbParameterType::hlbool => Ok(ParamType::Boolean),
            FbParameterType::hlvecbytes => Ok(ParamType::VecBytes),
            _ => bail!("Unknown parameter type: {:?}", value),
        }
    }
}

impl From<ParamType> for FbParameterType {
    fn from(value: ParamType) -> Self {
        match value {
            ParamType::Int => FbParameterType::hlint,
            ParamType::Long => FbParameterType::hllong,
            ParamType::String => FbParameterType::hlstring,
            ParamType::Boolean => FbParameterType::hlbool,
            ParamType::VecBytes => FbParameterType::hlvecbytes,
        }
    }
}
