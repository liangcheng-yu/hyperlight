use crate::flatbuffers::hyperlight::generated::{
    hlbool, hlboolArgs, hlint, hlintArgs, hllong, hllongArgs, hlsizeprefixedbuffer,
    hlsizeprefixedbufferArgs, hlstring, hlstringArgs, hlvoid, hlvoidArgs,
    size_prefixed_root_as_function_call_result, FunctionCallResult as FBFunctionCallResult,
    FunctionCallResultArgs as FBFunctionCallResultArgs, ReturnValue,
};
use crate::mem::{layout::SandboxMemoryLayout, shared_mem::SharedMemory};
use anyhow::{anyhow, Result};
use std::convert::{TryFrom, TryInto};

/// This is the type and value of a result from a FunctionCall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionCallResult {
    /// Return value is a signed 32 bit integer.
    Int(i32),
    /// Return value is a signed 64 bit integer.
    Long(i64),
    /// Return value is a boolean.
    Boolean(bool),
    /// Return value is a string.
    String(Option<String>),
    /// Return value is void.
    Void,
    /// Return value is a size prefixed vector of bytes.
    SizePrefixedBuffer(Option<Vec<u8>>),
}

impl FunctionCallResult {
    pub(crate) fn write_to_memory(
        &self,
        shared_mem: &mut SharedMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<()> {
        let input_data_offset = layout.input_data_buffer_offset;
        let function_call_buffer = Vec::<u8>::try_from(self)?;
        shared_mem.copy_from_slice(function_call_buffer.as_slice(), input_data_offset)
    }
}

impl TryFrom<&[u8]> for FunctionCallResult {
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        let function_call_result_fb =
            size_prefixed_root_as_function_call_result(value).map_err(|e| anyhow!(e))?;
        let function_call_result = match function_call_result_fb.return_value_type() {
            ReturnValue::hlint => {
                let hlint = function_call_result_fb
                    .return_value_as_hlint()
                    .ok_or_else(|| anyhow!("Failed to get hlint from return value"))?;
                FunctionCallResult::Int(hlint.value())
            }
            ReturnValue::hllong => {
                let hllong = function_call_result_fb
                    .return_value_as_hllong()
                    .ok_or_else(|| anyhow!("Failed to get hlong from return value"))?;
                FunctionCallResult::Long(hllong.value())
            }
            ReturnValue::hlbool => {
                let hlbool = function_call_result_fb
                    .return_value_as_hlbool()
                    .ok_or_else(|| anyhow!("Failed to get hlbool from return value"))?;
                FunctionCallResult::Boolean(hlbool.value())
            }
            ReturnValue::hlstring => {
                let hlstring = match function_call_result_fb.return_value_as_hlstring() {
                    Some(hlstring) => hlstring.value().map(|v| v.to_string()),
                    None => None,
                };
                FunctionCallResult::String(hlstring)
            }
            ReturnValue::hlvoid => FunctionCallResult::Void,
            ReturnValue::hlsizeprefixedbuffer => {
                let hlvecbytes =
                    match function_call_result_fb.return_value_as_hlsizeprefixedbuffer() {
                        Some(hlvecbytes) => hlvecbytes
                            .value()
                            .map(|val| val.iter().collect::<Vec<u8>>()),
                        None => None,
                    };
                FunctionCallResult::SizePrefixedBuffer(hlvecbytes)
            }
            _ => {
                return Err(anyhow!(
                    "Unknown return value type: {:?}",
                    function_call_result_fb.return_value_type()
                ))
            }
        };
        Ok(function_call_result)
    }
}

impl TryFrom<&FunctionCallResult> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: &FunctionCallResult) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let result = match value {
            FunctionCallResult::Int(i) => {
                let hlint = hlint::create(&mut builder, &hlintArgs { value: *i });
                let function_call_result = FBFunctionCallResult::create(
                    &mut builder,
                    &FBFunctionCallResultArgs {
                        return_value: Some(hlint.as_union_value()),
                        return_value_type: ReturnValue::hlint,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            FunctionCallResult::Long(l) => {
                let hllong = hllong::create(&mut builder, &hllongArgs { value: *l });
                let function_call_result = FBFunctionCallResult::create(
                    &mut builder,
                    &FBFunctionCallResultArgs {
                        return_value: Some(hllong.as_union_value()),
                        return_value_type: ReturnValue::hllong,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            FunctionCallResult::Boolean(b) => {
                let hlbool = hlbool::create(&mut builder, &hlboolArgs { value: *b });
                let function_call_result = FBFunctionCallResult::create(
                    &mut builder,
                    &FBFunctionCallResultArgs {
                        return_value: Some(hlbool.as_union_value()),
                        return_value_type: ReturnValue::hlbool,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            FunctionCallResult::String(s) => {
                let hlstring = match &s {
                    Some(s) => {
                        let val = builder.create_string(s.as_str());
                        hlstring::create(&mut builder, &hlstringArgs { value: Some(val) })
                    }
                    None => hlstring::create(&mut builder, &hlstringArgs { value: None }),
                };
                let function_call_result = FBFunctionCallResult::create(
                    &mut builder,
                    &FBFunctionCallResultArgs {
                        return_value: Some(hlstring.as_union_value()),
                        return_value_type: ReturnValue::hlstring,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            FunctionCallResult::SizePrefixedBuffer(v) => {
                let hlvecbytes = match &v {
                    Some(v) => {
                        let val = builder.create_vector(v.as_slice());
                        hlsizeprefixedbuffer::create(
                            &mut builder,
                            &hlsizeprefixedbufferArgs {
                                value: Some(val),
                                size_: v.len() as i32,
                            },
                        )
                    }
                    None => hlsizeprefixedbuffer::create(
                        &mut builder,
                        &hlsizeprefixedbufferArgs {
                            value: None,
                            size_: 0,
                        },
                    ),
                };
                let function_call_result = FBFunctionCallResult::create(
                    &mut builder,
                    &FBFunctionCallResultArgs {
                        return_value: Some(hlvecbytes.as_union_value()),
                        return_value_type: ReturnValue::hlsizeprefixedbuffer,
                    },
                );
                builder.finish_size_prefixed(function_call_result, None);
                builder.finished_data().to_vec()
            }
            FunctionCallResult::Void => {
                let hlvoid = hlvoid::create(&mut builder, &hlvoidArgs {});
                let function_call_result = FBFunctionCallResult::create(
                    &mut builder,
                    &FBFunctionCallResultArgs {
                        return_value: Some(hlvoid.as_union_value()),
                        return_value_type: ReturnValue::hlvoid,
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
            anyhow::bail!("The capacity of the vector is for FunctionCall is incorrect");
        }

        Ok(result)
    }
}

impl TryFrom<(&SharedMemory, &SandboxMemoryLayout)> for FunctionCallResult {
    type Error = anyhow::Error;
    fn try_from(value: (&SharedMemory, &SandboxMemoryLayout)) -> Result<Self> {
        // Get the size of the flatbuffer buffer from memory

        let fb_buffer_size = {
            let size_i32 = value.0.read_i32(value.1.output_data_buffer_offset)? + 4;
            usize::try_from(size_i32)
                .map_err(|_| anyhow!("could not convert buffer size i32 ({}) to usize", size_i32))
        }?;

        let mut function_call_result_buffer = vec![0; fb_buffer_size];
        value.0.copy_to_slice(
            &mut function_call_result_buffer,
            value.1.output_data_buffer_offset,
        )?;
        FunctionCallResult::try_from(function_call_result_buffer.as_slice())
    }
}

impl TryFrom<FunctionCallResult> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: FunctionCallResult) -> Result<Vec<u8>> {
        (&value).try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use hex_literal::hex;

    #[test]
    fn read_from_flatbuffer() -> Result<()> {
        let test_data = get_function_call_result_test_data();
        let function_call_result = FunctionCallResult::try_from(test_data.as_slice())?;
        let function_call_result1 = FunctionCallResult::String(Some("Hello, World!!".to_string()));
        assert_eq!(function_call_result, function_call_result1);
        Ok(())
    }

    #[test]
    fn write_to_flatbuffer() -> Result<()> {
        let function_call_result = FunctionCallResult::String(Some("Hello, World!!".to_string()));
        let function_call_result_buffer: Vec<u8> = function_call_result.try_into()?;
        assert_eq!(
            function_call_result_buffer,
            get_function_call_result_test_data()
        );

        Ok(())
    }

    // The test data is a valid flatbuffers buffer representing a function call result that contains a string result value of "Hello, World!!" as follows:
    // int HostMethod1(string="Hello from GuestFunction1, Hello from CallbackTest")
    pub(crate) fn get_function_call_result_test_data() -> Vec<u8> {
        hex!("3c0000000c00000008000e000700080008000000000000030c000000000006000800040006000000040000000e00000048656c6c6f2c20576f726c6421210000").to_vec()
    }
}
