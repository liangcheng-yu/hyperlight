#![allow(non_snake_case)]

use crate::error::HyperlightError::{
    self, FailedToGetValueFromParameter, UnexpectedFlatBufferReturnValueType,
    UnexpectedParameterValueType, UnexpectedReturnValueType, UnknownFlatBufferParameterType,
    UnknownFlatBufferParameterValue, UnknownFlatBufferReturnType, VectorCapacityInCorrect,
};
use crate::Result;
use crate::mem::mgr::SandboxMemoryManager;
use crate::{
    flatbuffers::hyperlight::generated::{
        hlbool, hlboolArgs, hlint, hlintArgs, hllong, hllongArgs, hlsizeprefixedbuffer,
        hlsizeprefixedbufferArgs, hlstring, hlstringArgs, hlvoid, hlvoidArgs,
        size_prefixed_root_as_function_call_result, FunctionCallResult as FbFunctionCallResult,
        FunctionCallResultArgs as FbFunctionCallResultArgs, Parameter,
        ParameterType as FbParameterType, ParameterValue as FbParameterValue,
        ReturnType as FbReturnType, ReturnValue as FbReturnValue,
    },
    mem::{layout::SandboxMemoryLayout, shared_mem::SharedMemory},
};
use crate::{log_then_return, new_error};

/// Supported parameter types with values for function calling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterValue {
    /// i32
    Int(i32),
    /// i64
    Long(i64),
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
    /// i64
    Long,
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
    /// i64
    Long(i64),
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
    /// i64
    Long,
    /// String
    String,
    /// bool
    Bool,
    /// ()
    Void,
    /// Vec<u8>
    VecBytes,
}

impl TryFrom<Parameter<'_>> for ParameterValue {
    type Error = HyperlightError;

    fn try_from(param: Parameter<'_>) -> Result<Self> {
        let value = param.value_type();
        let result = match value {
            FbParameterValue::hlint => param
                .value_as_hlint()
                .map(|hlint| ParameterValue::Int(hlint.value())),
            FbParameterValue::hllong => param
                .value_as_hllong()
                .map(|hllong| ParameterValue::Long(hllong.value())),
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
                log_then_return!(UnknownFlatBufferParameterValue(other));
            }
        };
        result.ok_or_else(FailedToGetValueFromParameter)
    }
}

impl From<ParameterType> for FbParameterType {
    fn from(value: ParameterType) -> Self {
        match value {
            ParameterType::Int => FbParameterType::hlint,
            ParameterType::Long => FbParameterType::hllong,
            ParameterType::String => FbParameterType::hlstring,
            ParameterType::Bool => FbParameterType::hlbool,
            ParameterType::VecBytes => FbParameterType::hlvecbytes,
        }
    }
}

impl From<ReturnType> for FbReturnType {
    fn from(value: ReturnType) -> Self {
        match value {
            ReturnType::Int => FbReturnType::hlint,
            ReturnType::Long => FbReturnType::hllong,
            ReturnType::String => FbReturnType::hlstring,
            ReturnType::Bool => FbReturnType::hlbool,
            ReturnType::Void => FbReturnType::hlvoid,
            ReturnType::VecBytes => FbReturnType::hlsizeprefixedbuffer,
        }
    }
}

impl TryFrom<FbParameterType> for ParameterType {
    type Error = HyperlightError;
    fn try_from(value: FbParameterType) -> Result<Self> {
        match value {
            FbParameterType::hlint => Ok(ParameterType::Int),
            FbParameterType::hllong => Ok(ParameterType::Long),
            FbParameterType::hlstring => Ok(ParameterType::String),
            FbParameterType::hlbool => Ok(ParameterType::Bool),
            FbParameterType::hlvecbytes => Ok(ParameterType::VecBytes),
            _ => {
                log_then_return!(UnknownFlatBufferParameterType(value));
            }
        }
    }
}

impl TryFrom<FbReturnType> for ReturnType {
    type Error = HyperlightError;
    fn try_from(value: FbReturnType) -> Result<Self> {
        match value {
            FbReturnType::hlint => Ok(ReturnType::Int),
            FbReturnType::hllong => Ok(ReturnType::Long),
            FbReturnType::hlstring => Ok(ReturnType::String),
            FbReturnType::hlbool => Ok(ReturnType::Bool),
            FbReturnType::hlvoid => Ok(ReturnType::Void),
            FbReturnType::hlsizeprefixedbuffer => Ok(ReturnType::VecBytes),
            _ => {
                log_then_return!(UnknownFlatBufferReturnType(value));
            }
        }
    }
}

impl TryFrom<ParameterValue> for i32 {
    type Error = HyperlightError;
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::Int(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedParameterValueType(
                    value.clone(),
                    "i32".to_string()
                ));
            }
        }
    }
}

impl TryFrom<ParameterValue> for i64 {
    type Error = HyperlightError;
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::Long(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedParameterValueType(
                    value.clone(),
                    String::from("i64")
                ));
            }
        }
    }
}

impl TryFrom<ParameterValue> for String {
    type Error = HyperlightError;
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::String(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedParameterValueType(
                    value.clone(),
                    String::from("String")
                ));
            }
        }
    }
}

impl TryFrom<ParameterValue> for bool {
    type Error = HyperlightError;
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::Bool(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedParameterValueType(
                    value.clone(),
                    String::from("bool")
                ));
            }
        }
    }
}

impl TryFrom<ParameterValue> for Vec<u8> {
    type Error = HyperlightError;
    fn try_from(value: ParameterValue) -> Result<Self> {
        match value {
            ParameterValue::VecBytes(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedParameterValueType(
                    value.clone(),
                    String::from("Vec<u8>")
                ));
            }
        }
    }
}

impl TryFrom<ReturnValue> for i32 {
    type Error = HyperlightError;
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::Int(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedReturnValueType(
                    value.clone(),
                    String::from("i32")
                ));
            }
        }
    }
}

impl TryFrom<ReturnValue> for i64 {
    type Error = HyperlightError;
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::Long(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedReturnValueType(
                    value.clone(),
                    String::from("i64")
                ));
            }
        }
    }
}

impl TryFrom<ReturnValue> for String {
    type Error = HyperlightError;
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::String(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedReturnValueType(
                    value.clone(),
                    String::from("String")
                ));
            }
        }
    }
}

impl TryFrom<ReturnValue> for bool {
    type Error = HyperlightError;
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::Bool(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedReturnValueType(
                    value.clone(),
                    String::from("bool")
                ));
            }
        }
    }
}

impl TryFrom<ReturnValue> for Vec<u8> {
    type Error = HyperlightError;
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::VecBytes(v) => Ok(v),
            _ => {
                log_then_return!(UnexpectedReturnValueType(
                    value.clone(),
                    String::from("Vec<u8>")
                ));
            }
        }
    }
}

impl TryFrom<ReturnValue> for () {
    type Error = HyperlightError;
    fn try_from(value: ReturnValue) -> Result<Self> {
        match value {
            ReturnValue::Void => Ok(()),
            _ => {
                log_then_return!(UnexpectedReturnValueType(value.clone(), String::from("()")));
            }
        }
    }
}

impl ReturnValue {
    pub(crate) fn write_to_memory(
        &self,
        shared_mem: &mut SharedMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<()> {
        let input_data_offset = layout.input_data_buffer_offset;
        let function_call_ret_val_buffer = Vec::<u8>::try_from(self)?;
        shared_mem.copy_from_slice(function_call_ret_val_buffer.as_slice(), input_data_offset)
    }
}

impl TryFrom<FbFunctionCallResult<'_>> for ReturnValue {
    type Error = HyperlightError;
    fn try_from(function_call_result_fb: FbFunctionCallResult<'_>) -> Result<Self> {
        match function_call_result_fb.return_value_type() {
            FbReturnValue::hlint => {
                let hlint = function_call_result_fb
                    .return_value_as_hlint()
                    .ok_or_else(|| new_error!("Failed to get hlint from return value"))?;
                Ok(ReturnValue::Int(hlint.value()))
            }
            FbReturnValue::hllong => {
                let hllong = function_call_result_fb
                    .return_value_as_hllong()
                    .ok_or_else(|| new_error!("Failed to get hlong from return value"))?;
                Ok(ReturnValue::Long(hllong.value()))
            }
            FbReturnValue::hlbool => {
                let hlbool = function_call_result_fb
                    .return_value_as_hlbool()
                    .ok_or_else(|| new_error!("Failed to get hlbool from return value"))?;
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
                Ok(ReturnValue::VecBytes(hlvecbytes.unwrap_or(vec![])))
            }
            other => {
                log_then_return!(UnexpectedFlatBufferReturnValueType(other));
            }
        }
    }
}

impl TryFrom<&[u8]> for ReturnValue {
    type Error = HyperlightError;
    fn try_from(value: &[u8]) -> Result<Self> {
        let function_call_result_fb = size_prefixed_root_as_function_call_result(value)?;
        function_call_result_fb.try_into()
    }
}

impl TryFrom<&ReturnValue> for Vec<u8> {
    type Error = HyperlightError;
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

        // This vector may be converted to a raw pointer and returned via the C API and the C API uses the size prefix to determine the capacity and length of the buffer in order to free the memory  , therefore:
        // 1. the capacity of the vector should be the same as the length
        // 2. the capacity of the vector should be the same as the size of the buffer (from the size prefix) + 4 bytes (the size of the size prefix field is not included in the size)

        let length = unsafe { flatbuffers::read_scalar::<i32>(&result[..4]) };
        if result.capacity() != result.len() || result.capacity() != length as usize + 4 {
            log_then_return!(VectorCapacityInCorrect(
                result.capacity(),
                result.len(),
                length + 4
            ));
        }

        Ok(result)
    }
}

impl TryFrom<&SandboxMemoryManager> for ReturnValue {
    type Error = HyperlightError;
    fn try_from(mgr: &SandboxMemoryManager) -> Result<Self> {
        let value: (&SharedMemory, &SandboxMemoryLayout) = (&mgr.shared_mem, &mgr.layout);

        let fb_buffer_size = {
            let size_i32 = value.0.read_i32(value.1.output_data_buffer_offset)? + 4;
            usize::try_from(size_i32)
        }?;

        let mut function_call_result_buffer = vec![0; fb_buffer_size];
        value.0.copy_to_slice(
            &mut function_call_result_buffer,
            value.1.output_data_buffer_offset,
        )?;
        ReturnValue::try_from(function_call_result_buffer.as_slice())
    }
}