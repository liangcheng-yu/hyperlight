use alloc::string::{String, ToString};
use alloc::vec::Vec;

use anyhow::{anyhow, bail, Error, Result};
#[cfg(feature = "tracing")]
use tracing::{instrument, Span};

use crate::flatbuffers::hyperlight::generated::{
    hlbool, hlboolArgs, hlint, hlintArgs, hllong, hllongArgs, hlsizeprefixedbuffer,
    hlsizeprefixedbufferArgs, hlstring, hlstringArgs, hluint, hluintArgs, hlulong, hlulongArgs,
    hlvoid, hlvoidArgs, size_prefixed_root_as_function_call_result,
    FunctionCallResult as FbFunctionCallResult, FunctionCallResultArgs as FbFunctionCallResultArgs,
    Parameter, ParameterType as FbParameterType, ParameterValue as FbParameterValue,
    ReturnType as FbReturnType, ReturnValue as FbReturnValue,
};

/// Supported parameter types with values for function calling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterValue {
    /// i32
    Int(i32),
    /// u32
    UInt(u32),
    /// i64
    Long(i64),
    /// i64
    ULong(u64),
    /// String
    String(String),
    /// bool
    Bool(bool),
    /// Vec<u8>
    VecBytes(Vec<u8>),
}

/// Supported parameter types for function calling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterType {
    /// i32
    Int,
    /// u32
    UInt,
    /// i64
    Long,
    /// u64
    ULong,
    /// String
    String,
    /// bool
    Bool,
    /// Vec<u8>
    VecBytes,
}

/// Supported return types with values from function calling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnValue {
    /// i32
    Int(i32),
    /// u32
    UInt(u32),
    /// i64
    Long(i64),
    /// u64
    ULong(u64),
    /// String
    String(String),
    /// bool
    Bool(bool),
    /// ()
    Void,
    /// Vec<u8>
    VecBytes(Vec<u8>),
}

/// Supported return types from function calling.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ReturnType {
    /// i32
    #[default]
    Int,
    /// u32
    UInt,
    /// i64
    Long,
    /// u64
    ULong,
    /// String
    String,
    /// bool
    Bool,
    /// ()
    Void,
    /// Vec<u8>
    VecBytes,
}

impl From<&ParameterValue> for ParameterType {
    #[cfg_attr(feature = "tracing", instrument(skip_all, parent = Span::current(), level= "Trace"))]
    fn from(value: &ParameterValue) -> Self {
        match *value {
            ParameterValue::Int(_) => ParameterType::Int,
            ParameterValue::UInt(_) => ParameterType::UInt,
            ParameterValue::Long(_) => ParameterType::Long,
            ParameterValue::ULong(_) => ParameterType::ULong,
            ParameterValue::String(_) => ParameterType::String,
            ParameterValue::Bool(_) => ParameterType::Bool,
            ParameterValue::VecBytes(_) => ParameterType::VecBytes,
        }
    }
}

impl TryFrom<Parameter<'_>> for ParameterValue {
    type Error = Error;

    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(param: Parameter<'_>) -> Result<Self> {
        let value = param.value_type();
        let result = match value {
            FbParameterValue::hlint => param
                .value_as_hlint()
                .map(|hlint| ParameterValue::Int(hlint.value())),
            FbParameterValue::hluint => param
                .value_as_hluint()
                .map(|hluint| ParameterValue::UInt(hluint.value())),
            FbParameterValue::hllong => param
                .value_as_hllong()
                .map(|hllong| ParameterValue::Long(hllong.value())),
            FbParameterValue::hlulong => param
                .value_as_hlulong()
                .map(|hlulong| ParameterValue::ULong(hlulong.value())),
            FbParameterValue::hlbool => param
                .value_as_hlbool()
                .map(|hlbool| ParameterValue::Bool(hlbool.value())),
            FbParameterValue::hlstring => param.value_as_hlstring().map(|hlstring| {
                ParameterValue::String(hlstring.value().unwrap_or_default().to_string())
            }),
            FbParameterValue::hlvecbytes => param.value_as_hlvecbytes().map(|hlvecbytes| {
                ParameterValue::VecBytes(hlvecbytes.value().unwrap_or_default().iter().collect())
            }),
            other => {
                bail!("Unexpected flatbuffer parameter value type: {:?}", other);
            }
        };
        result.ok_or_else(|| anyhow!("Failed to get parameter value"))
    }
}

impl From<ParameterType> for FbParameterType {
    #[cfg_attr(feature = "tracing", instrument(skip_all, parent = Span::current(), level= "Trace"))]
    fn from(value: ParameterType) -> Self {
        match value {
            ParameterType::Int => FbParameterType::hlint,
            ParameterType::UInt => FbParameterType::hluint,
            ParameterType::Long => FbParameterType::hllong,
            ParameterType::ULong => FbParameterType::hlulong,
            ParameterType::String => FbParameterType::hlstring,
            ParameterType::Bool => FbParameterType::hlbool,
            ParameterType::VecBytes => FbParameterType::hlvecbytes,
        }
    }
}

impl From<ReturnType> for FbReturnType {
    #[cfg_attr(feature = "tracing", instrument(skip_all, parent = Span::current(), level= "Trace"))]
    fn from(value: ReturnType) -> Self {
        match value {
            ReturnType::Int => FbReturnType::hlint,
            ReturnType::UInt => FbReturnType::hluint,
            ReturnType::Long => FbReturnType::hllong,
            ReturnType::ULong => FbReturnType::hlulong,
            ReturnType::String => FbReturnType::hlstring,
            ReturnType::Bool => FbReturnType::hlbool,
            ReturnType::Void => FbReturnType::hlvoid,
            ReturnType::VecBytes => FbReturnType::hlsizeprefixedbuffer,
        }
    }
}

impl TryFrom<FbParameterType> for ParameterType {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: FbParameterType) -> Result<Self> {
        match value {
            FbParameterType::hlint => Ok(ParameterType::Int),
            FbParameterType::hluint => Ok(ParameterType::UInt),
            FbParameterType::hllong => Ok(ParameterType::Long),
            FbParameterType::hlulong => Ok(ParameterType::ULong),
            FbParameterType::hlstring => Ok(ParameterType::String),
            FbParameterType::hlbool => Ok(ParameterType::Bool),
            FbParameterType::hlvecbytes => Ok(ParameterType::VecBytes),
            _ => {
                bail!("Unexpected flatbuffer parameter type: {:?}", value)
            }
        }
    }
}

impl TryFrom<FbReturnType> for ReturnType {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: FbReturnType) -> Result<Self> {
        match value {
            FbReturnType::hlint => Ok(ReturnType::Int),
            FbReturnType::hluint => Ok(ReturnType::UInt),
            FbReturnType::hllong => Ok(ReturnType::Long),
            FbReturnType::hlulong => Ok(ReturnType::ULong),
            FbReturnType::hlstring => Ok(ReturnType::String),
            FbReturnType::hlbool => Ok(ReturnType::Bool),
            FbReturnType::hlvoid => Ok(ReturnType::Void),
            FbReturnType::hlsizeprefixedbuffer => Ok(ReturnType::VecBytes),
            _ => {
                bail!("Unexpected flatbuffer return type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ParameterValue> for i32 {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::Int(v) => Ok(v),
            _ => {
                bail!("Unexpected parameter value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ParameterValue> for u32 {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::UInt(v) => Ok(v),
            _ => {
                bail!("Unexpected parameter value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ParameterValue> for i64 {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::Long(v) => Ok(v),
            _ => {
                bail!("Unexpected parameter value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ParameterValue> for u64 {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::ULong(v) => Ok(v),
            _ => {
                bail!("Unexpected parameter value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ParameterValue> for String {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::String(v) => Ok(v),
            _ => {
                bail!("Unexpected parameter value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ParameterValue> for bool {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::Bool(v) => Ok(v),
            _ => {
                bail!("Unexpected parameter value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ParameterValue> for Vec<u8> {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::VecBytes(v) => Ok(v),
            _ => {
                bail!("Unexpected parameter value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ReturnValue> for i32 {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::Int(v) => Ok(v),
            _ => {
                bail!("Unexpected return value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ReturnValue> for u32 {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::UInt(v) => Ok(v),
            _ => {
                bail!("Unexpected return value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ReturnValue> for i64 {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::Long(v) => Ok(v),
            _ => {
                bail!("Unexpected return value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ReturnValue> for u64 {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::ULong(v) => Ok(v),
            _ => {
                bail!("Unexpected return value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ReturnValue> for String {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::String(v) => Ok(v),
            _ => {
                bail!("Unexpected return value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ReturnValue> for bool {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::Bool(v) => Ok(v),
            _ => {
                bail!("Unexpected return value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ReturnValue> for Vec<u8> {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::VecBytes(v) => Ok(v),
            _ => {
                bail!("Unexpected return value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<ReturnValue> for () {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::Void => Ok(()),
            _ => {
                bail!("Unexpected return value type: {:?}", value)
            }
        }
    }
}

impl TryFrom<FbFunctionCallResult<'_>> for ReturnValue {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(function_call_result_fb: FbFunctionCallResult<'_>) -> Result<Self> {
        match function_call_result_fb.return_value_type() {
            FbReturnValue::hlint => {
                let hlint = function_call_result_fb
                    .return_value_as_hlint()
                    .ok_or_else(|| anyhow!("Failed to get hlint from return value"))?;
                Ok(ReturnValue::Int(hlint.value()))
            }
            FbReturnValue::hluint => {
                let hluint = function_call_result_fb
                    .return_value_as_hluint()
                    .ok_or_else(|| anyhow!("Failed to get hluint from return value"))?;
                Ok(ReturnValue::UInt(hluint.value()))
            }
            FbReturnValue::hllong => {
                let hllong = function_call_result_fb
                    .return_value_as_hllong()
                    .ok_or_else(|| anyhow!("Failed to get hllong from return value"))?;
                Ok(ReturnValue::Long(hllong.value()))
            }
            FbReturnValue::hlulong => {
                let hlulong = function_call_result_fb
                    .return_value_as_hlulong()
                    .ok_or_else(|| anyhow!("Failed to get hlulong from return value"))?;
                Ok(ReturnValue::ULong(hlulong.value()))
            }
            FbReturnValue::hlbool => {
                let hlbool = function_call_result_fb
                    .return_value_as_hlbool()
                    .ok_or_else(|| anyhow!("Failed to get hlbool from return value"))?;
                Ok(ReturnValue::Bool(hlbool.value()))
            }
            FbReturnValue::hlstring => {
                let hlstring = match function_call_result_fb.return_value_as_hlstring() {
                    Some(hlstring) => hlstring.value().map(|v| v.to_string()),
                    None => None,
                };
                Ok(ReturnValue::String(hlstring.unwrap_or("".to_string())))
            }
            FbReturnValue::hlvoid => Ok(ReturnValue::Void),
            FbReturnValue::hlsizeprefixedbuffer => {
                let hlvecbytes =
                    match function_call_result_fb.return_value_as_hlsizeprefixedbuffer() {
                        Some(hlvecbytes) => hlvecbytes
                            .value()
                            .map(|val| val.iter().collect::<Vec<u8>>()),
                        None => None,
                    };
                Ok(ReturnValue::VecBytes(hlvecbytes.unwrap_or(Vec::new())))
            }
            other => {
                bail!("Unexpected flatbuffer return value type: {:?}", other)
            }
        }
    }
}

impl TryFrom<&[u8]> for ReturnValue {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: &[u8]) -> Result<Self> {
        let function_call_result_fb = size_prefixed_root_as_function_call_result(value)
            .map_err(|e| anyhow!("Failed to get ReturnValue from bytes: {:?}", e))?;
        function_call_result_fb.try_into()
    }
}

impl TryFrom<&ReturnValue> for Vec<u8> {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: &ReturnValue) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let result = match value {
            ReturnValue::Int(i) => {
                let hlint = hlint::create(&mut builder, &hlintArgs { value: *i });
                let function_call_result = FbFunctionCallResult::create(
                    &mut builder,
                    &FbFunctionCallResultArgs {
                        return_value: Some(hlint.as_union_value()),
                        return_value_type: FbReturnValue::hlint,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            ReturnValue::UInt(ui) => {
                let hluint = hluint::create(&mut builder, &hluintArgs { value: *ui });
                let function_call_result = FbFunctionCallResult::create(
                    &mut builder,
                    &FbFunctionCallResultArgs {
                        return_value: Some(hluint.as_union_value()),
                        return_value_type: FbReturnValue::hluint,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            ReturnValue::Long(l) => {
                let hllong = hllong::create(&mut builder, &hllongArgs { value: *l });
                let function_call_result = FbFunctionCallResult::create(
                    &mut builder,
                    &FbFunctionCallResultArgs {
                        return_value: Some(hllong.as_union_value()),
                        return_value_type: FbReturnValue::hllong,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            ReturnValue::ULong(ul) => {
                let hlulong = hlulong::create(&mut builder, &hlulongArgs { value: *ul });
                let function_call_result = FbFunctionCallResult::create(
                    &mut builder,
                    &FbFunctionCallResultArgs {
                        return_value: Some(hlulong.as_union_value()),
                        return_value_type: FbReturnValue::hlulong,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            ReturnValue::Bool(b) => {
                let hlbool = hlbool::create(&mut builder, &hlboolArgs { value: *b });
                let function_call_result = FbFunctionCallResult::create(
                    &mut builder,
                    &FbFunctionCallResultArgs {
                        return_value: Some(hlbool.as_union_value()),
                        return_value_type: FbReturnValue::hlbool,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            ReturnValue::String(s) => {
                let hlstring = {
                    let val = builder.create_string(s.as_str());
                    hlstring::create(&mut builder, &hlstringArgs { value: Some(val) })
                };
                let function_call_result = FbFunctionCallResult::create(
                    &mut builder,
                    &FbFunctionCallResultArgs {
                        return_value: Some(hlstring.as_union_value()),
                        return_value_type: FbReturnValue::hlstring,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            ReturnValue::VecBytes(v) => {
                let hlvecbytes = {
                    let val = builder.create_vector(v.as_slice());
                    hlsizeprefixedbuffer::create(
                        &mut builder,
                        &hlsizeprefixedbufferArgs {
                            value: Some(val),
                            size_: v.len() as i32,
                        },
                    )
                };
                let function_call_result = FbFunctionCallResult::create(
                    &mut builder,
                    &FbFunctionCallResultArgs {
                        return_value: Some(hlvecbytes.as_union_value()),
                        return_value_type: FbReturnValue::hlsizeprefixedbuffer,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            ReturnValue::Void => {
                let hlvoid = hlvoid::create(&mut builder, &hlvoidArgs {});
                let function_call_result = FbFunctionCallResult::create(
                    &mut builder,
                    &FbFunctionCallResultArgs {
                        return_value: Some(hlvoid.as_union_value()),
                        return_value_type: FbReturnValue::hlvoid,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
        };

        Ok(result)
    }
}
