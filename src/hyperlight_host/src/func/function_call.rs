extern crate flatbuffers;
use crate::flatbuffers::hyperlight::generated::{
    hlbool, hlboolArgs, hlint, hlintArgs, hllong, hllongArgs, hlstring, hlstringArgs, hlvecbytes,
    hlvecbytesArgs, size_prefixed_root_as_function_call, FunctionCall as FBFunctionCall,
    FunctionCallArgs as FBFunctionCallArgs, FunctionCallType as FBFunctionCallType, Parameter,
    ParameterArgs, ParameterValue as FbParameterValue, ReturnType as FbReturnType,
};
use crate::func::function_types::ParameterValue;
use crate::mem::layout::SandboxMemoryLayout;
use crate::mem::shared_mem::SharedMemory;
use anyhow::{anyhow, Result};
use flatbuffers::WIPOffset;
use readonly;
use std::convert::{TryFrom, TryInto};

/// `Functioncall` represents a call to a function in the guest or host.
#[readonly::make]
pub struct FunctionCall {
    /// The function name
    pub function_name: String,
    /// The parameters for the function call.
    pub parameters: Option<Vec<ParameterValue>>,
    function_call_type: FunctionCallType,
    expected_return_type: ExpectedFunctionCallReturnType,
}

/// The type of function call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionCallType {
    /// The function call is to a guest function.
    Guest,
    /// The function call is to a host function.
    Host,
}

/// The expcted return type of the function call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedFunctionCallReturnType {
    /// The expected return type is Int.
    Int,
    /// The expected return type is Long.
    Long,
    /// The expected return type is String.
    String,
    /// The expected return type is a vec prefixed with a 4 byte size.
    SizePrefixedBuffer,
    /// The expected return type is void.
    Void,
    /// The expected return type is a boolean.
    Bool,
}

impl TryFrom<&[u8]> for FunctionCall {
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        let guest_function_call_fb =
            size_prefixed_root_as_function_call(value).map_err(|e| anyhow!(e))?;
        let function_name = guest_function_call_fb.function_name();
        let function_call_type = match guest_function_call_fb.function_call_type() {
            FBFunctionCallType::guest => FunctionCallType::Guest,
            FBFunctionCallType::host => FunctionCallType::Host,
            _ => {
                anyhow::bail!("Unknown function call type");
            }
        };
        let expected_return_type = match guest_function_call_fb.expected_return_type() {
            FbReturnType::hlint => ExpectedFunctionCallReturnType::Int,
            FbReturnType::hllong => ExpectedFunctionCallReturnType::Long,
            FbReturnType::hlstring => ExpectedFunctionCallReturnType::String,
            FbReturnType::hlsizeprefixedbuffer => {
                ExpectedFunctionCallReturnType::SizePrefixedBuffer
            }
            FbReturnType::hlvoid => ExpectedFunctionCallReturnType::Void,
            FbReturnType::hlbool => ExpectedFunctionCallReturnType::Bool,
            _ => {
                anyhow::bail!("Unknown function call type");
            }
        };
        let parameters = match guest_function_call_fb.parameters() {
            Some(p) => {
                let len = p.len();
                let mut v: Vec<ParameterValue> = Vec::with_capacity(len);
                for i in 0..len {
                    let param = p.get(i);
                    let param_type = param.value_type();
                    match param_type {
                        FbParameterValue::hlint => {
                            let hlint = param.value_as_hlint().ok_or_else(|| {
                                anyhow!("Failed to get hlint from parameter {}", i)
                            })?;
                            v.push(ParameterValue::Int(hlint.value()));
                        }
                        FbParameterValue::hllong => {
                            let hllong = param.value_as_hllong().ok_or_else(|| {
                                anyhow!("Failed to get hlong from parameter {}", i)
                            })?;
                            v.push(ParameterValue::Long(hllong.value()));
                        }
                        FbParameterValue::hlbool => {
                            let hlbool = param.value_as_hlbool().ok_or_else(|| {
                                anyhow!("Failed to get hlbool from parameter {}", i)
                            })?;
                            v.push(ParameterValue::Bool(hlbool.value()));
                        }
                        FbParameterValue::hlstring => {
                            let hlstring = param.value_as_hlstring().ok_or_else(|| {
                                anyhow!("Failed to get hlstring from parameter {}", i)
                            })?;

                            v.push(ParameterValue::String(
                                hlstring.value().unwrap_or_default().to_string(),
                            ));
                        }
                        FbParameterValue::hlvecbytes => {
                            let hlvecbytes = param.value_as_hlvecbytes().ok_or_else(|| {
                                anyhow!("Failed to get hlvecbytes from parameter {}", i)
                            })?;
                            match hlvecbytes.value() {
                                Some(val) => v.push(ParameterValue::VecBytes(
                                    val.iter().collect::<Vec<u8>>(),
                                )),
                                None => v.push(ParameterValue::VecBytes(vec![])),
                            }
                        }
                        _ => {
                            anyhow::bail!("Unknown parameter type");
                        }
                    };
                }
                Some(v)
            }
            None => None,
        };
        Ok(Self {
            function_name: function_name.to_string(),
            parameters,
            function_call_type,
            expected_return_type,
        })
    }
}

impl TryFrom<&FunctionCall> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: &FunctionCall) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let function_name = builder.create_string(&value.function_name);

        let function_call_type = match value.function_call_type {
            FunctionCallType::Guest => FBFunctionCallType::guest,
            FunctionCallType::Host => FBFunctionCallType::host,
        };

        let expected_return_type = match value.expected_return_type {
            ExpectedFunctionCallReturnType::Int => FbReturnType::hlint,
            ExpectedFunctionCallReturnType::Long => FbReturnType::hllong,
            ExpectedFunctionCallReturnType::String => FbReturnType::hlstring,
            ExpectedFunctionCallReturnType::SizePrefixedBuffer => {
                FbReturnType::hlsizeprefixedbuffer
            }
            ExpectedFunctionCallReturnType::Void => FbReturnType::hlvoid,
            ExpectedFunctionCallReturnType::Bool => FbReturnType::hlbool,
        };

        let vec_parameters = match &value.parameters {
            Some(p) => {
                let num_items = p.len();
                let mut parameters: Vec<WIPOffset<Parameter>> = Vec::with_capacity(num_items);

                for param in p {
                    match param {
                        ParameterValue::Int(i) => {
                            let hlint = hlint::create(&mut builder, &hlintArgs { value: *i });
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: FbParameterValue::hlint,
                                    value: Some(hlint.as_union_value()),
                                },
                            );
                            parameters.push(parameter);
                        }
                        ParameterValue::Long(l) => {
                            let hllong = hllong::create(&mut builder, &hllongArgs { value: *l });
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: FbParameterValue::hllong,
                                    value: Some(hllong.as_union_value()),
                                },
                            );
                            parameters.push(parameter);
                        }
                        ParameterValue::Bool(b) => {
                            let hlbool: WIPOffset<hlbool<'_>> =
                                hlbool::create(&mut builder, &hlboolArgs { value: *b });
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: FbParameterValue::hlbool,
                                    value: Some(hlbool.as_union_value()),
                                },
                            );
                            parameters.push(parameter);
                        }
                        ParameterValue::String(s) => {
                            let hlstring = {
                                let val = builder.create_string(s.as_str());
                                hlstring::create(&mut builder, &hlstringArgs { value: Some(val) })
                            };
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: FbParameterValue::hlstring,
                                    value: Some(hlstring.as_union_value()),
                                },
                            );
                            parameters.push(parameter);
                        }
                        ParameterValue::VecBytes(v) => {
                            let vec_bytes = builder.create_vector(v);

                            //let vec_bytes = builder.create_vector(&v);

                            let hlvecbytes = hlvecbytes::create(
                                &mut builder,
                                &hlvecbytesArgs {
                                    value: Some(vec_bytes),
                                },
                            );
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: FbParameterValue::hlvecbytes,
                                    value: Some(hlvecbytes.as_union_value()),
                                },
                            );
                            parameters.push(parameter);
                        }
                    }
                }
                parameters
            }
            None => {
                let parameters: Vec<WIPOffset<Parameter>> = Vec::new();
                parameters
            }
        };

        let parameters = match vec_parameters.len() {
            0 => None,
            _ => Some(builder.create_vector(&vec_parameters)),
        };

        let function_call = FBFunctionCall::create(
            &mut builder,
            &FBFunctionCallArgs {
                function_name: Some(function_name),
                parameters,
                function_call_type,
                expected_return_type,
            },
        );
        builder.finish_size_prefixed(function_call, None);
        let res = builder.finished_data().to_vec();

        // This vector may be converted to a raw pointer and returned via the C API and the C API uses the size prefix to determine the capacity and length of the buffer in order to free the memory  , therefore:
        // 1. the capacity of the vector should be the same as the length
        // 2. the capacity of the vector should be the same as the size of the buffer (from the size prefix) + 4 bytes (the size of the size prefix field is not included in the size)

        let length = unsafe { flatbuffers::read_scalar::<i32>(&res[..4]) };
        if res.capacity() != res.len() || res.capacity() != length as usize + 4 {
            anyhow::bail!("The capacity of the vector is for FunctionCall is incorrect");
        }

        Ok(res)
    }
}

impl TryFrom<FunctionCall> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: FunctionCall) -> Result<Vec<u8>> {
        (&value).try_into()
    }
}

pub(crate) trait WriteFunctionCallToMemory {
    fn write(
        &self,
        function_call_buffer: &[u8],
        guest_mem: &mut SharedMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<()>;
}

pub(crate) trait ReadFunctionCallFromMemory {
    fn read(&self, guest_mem: &SharedMemory, layout: &SandboxMemoryLayout) -> Result<Vec<u8>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{get_guest_function_call_test_data, get_host_function_call_test_data};
    use anyhow::Result;

    #[test]
    fn read_from_flatbuffer() -> Result<()> {
        let test_data = get_guest_function_call_test_data();
        let function_call = FunctionCall::try_from(test_data.as_slice())?;
        assert_eq!(function_call.function_name, "PrintSevenArgs");
        assert!(function_call.parameters.is_some());
        let parameters = function_call.parameters.unwrap();
        assert_eq!(parameters.len(), 7);
        let expected_parameters = vec![
            ParameterValue::String(String::from("Test7")),
            ParameterValue::Int(8),
            ParameterValue::Long(9),
            ParameterValue::String(String::from("Tested")),
            ParameterValue::String(String::from("Test7")),
            ParameterValue::Bool(false),
            ParameterValue::Bool(true),
        ];
        assert!(expected_parameters == parameters);
        assert_eq!(function_call.function_call_type, FunctionCallType::Guest);

        let test_data = get_host_function_call_test_data();
        let function_call = FunctionCall::try_from(test_data.as_slice())?;
        assert_eq!(function_call.function_name, "HostMethod1");
        assert!(function_call.parameters.is_some());
        let parameters = function_call.parameters.unwrap();
        assert_eq!(parameters.len(), 1);
        let expected_parameters = vec![ParameterValue::String(String::from(
            "Hello from GuestFunction1, Hello from CallbackTest",
        ))];
        assert!(expected_parameters == parameters);
        assert_eq!(function_call.function_call_type, FunctionCallType::Host);

        Ok(())
    }

    #[test]
    fn write_to_flatbuffer() -> Result<()> {
        let guest_parameters = Some(vec![
            ParameterValue::String(String::from("Test7")),
            ParameterValue::Int(8),
            ParameterValue::Long(9),
            ParameterValue::String(String::from("Tested")),
            ParameterValue::String(String::from("Test7")),
            ParameterValue::Bool(false),
            ParameterValue::Bool(true),
        ]);
        let guest_function_call = FunctionCall {
            function_name: "PrintSevenArgs".to_string(),
            parameters: guest_parameters,
            function_call_type: FunctionCallType::Guest,
            expected_return_type: ExpectedFunctionCallReturnType::Int,
        };
        let guest_function_call_buffer: Vec<u8> = guest_function_call.try_into()?;
        assert_eq!(
            guest_function_call_buffer,
            get_guest_function_call_test_data()
        );

        let host_parameters = Some(vec![ParameterValue::String(String::from(
            "Hello from GuestFunction1, Hello from CallbackTest",
        ))]);
        let function_call = FunctionCall {
            function_name: "HostMethod1".to_string(),
            parameters: host_parameters,
            function_call_type: FunctionCallType::Host,
            expected_return_type: ExpectedFunctionCallReturnType::Int,
        };
        let host_function_call_buffer: Vec<u8> = function_call.try_into()?;
        assert_eq!(
            host_function_call_buffer,
            get_host_function_call_test_data()
        );

        Ok(())
    }
}
