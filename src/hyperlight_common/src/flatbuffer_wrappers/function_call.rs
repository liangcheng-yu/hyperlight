use alloc::string::{String, ToString};
use alloc::vec::Vec;

use anyhow::{bail, Error, Result};
use flatbuffers::WIPOffset;
#[cfg(feature = "tracing")]
use tracing::{instrument, Span};

use super::function_types::{ParameterValue, ReturnType};
use crate::flatbuffers::hyperlight::generated::{
    hlbool, hlboolArgs, hlint, hlintArgs, hllong, hllongArgs, hlstring, hlstringArgs, hluint,
    hluintArgs, hlulong, hlulongArgs, hlvecbytes, hlvecbytesArgs,
    size_prefixed_root_as_function_call, FunctionCall as FbFunctionCall,
    FunctionCallArgs as FbFunctionCallArgs, FunctionCallType as FbFunctionCallType, Parameter,
    ParameterArgs, ParameterValue as FbParameterValue,
};

/// The type of function call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionCallType {
    /// The function call is to a guest function.
    Guest,
    /// The function call is to a host function.
    Host,
}

/// `Functioncall` represents a call to a function in the guest or host.
#[derive(Clone)]
pub struct FunctionCall {
    /// The function name
    pub function_name: String,
    /// The parameters for the function call.
    pub parameters: Option<Vec<ParameterValue>>,
    function_call_type: FunctionCallType,
    expected_return_type: ReturnType,
}

impl FunctionCall {
    #[cfg_attr(feature = "tracing", instrument(skip_all, parent = Span::current(), level= "Trace"))]
    pub fn new(
        function_name: String,
        parameters: Option<Vec<ParameterValue>>,
        function_call_type: FunctionCallType,
        expected_return_type: ReturnType,
    ) -> Self {
        Self {
            function_name,
            parameters,
            function_call_type,
            expected_return_type,
        }
    }

    /// The type of the function call.
    pub fn function_call_type(&self) -> FunctionCallType {
        self.function_call_type.clone()
    }
}

#[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
pub fn validate_guest_function_call_buffer(function_call_buffer: &[u8]) -> Result<()> {
    let guest_function_call_fb = size_prefixed_root_as_function_call(function_call_buffer)
        .map_err(|e| anyhow::anyhow!("Error reading function call buffer: {:?}", e))?;
    match guest_function_call_fb.function_call_type() {
        FbFunctionCallType::guest => Ok(()),
        other => {
            bail!("Invalid function call type: {:?}", other);
        }
    }
}

#[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
pub fn validate_host_function_call_buffer(function_call_buffer: &[u8]) -> Result<()> {
    let host_function_call_fb = size_prefixed_root_as_function_call(function_call_buffer)
        .map_err(|e| anyhow::anyhow!("Error reading function call buffer: {:?}", e))?;
    match host_function_call_fb.function_call_type() {
        FbFunctionCallType::host => Ok(()),
        other => {
            bail!("Invalid function call type: {:?}", other);
        }
    }
}

impl TryFrom<&[u8]> for FunctionCall {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: &[u8]) -> Result<Self> {
        let function_call_fb = size_prefixed_root_as_function_call(value)
            .map_err(|e| anyhow::anyhow!("Error reading function call buffer: {:?}", e))?;
        let function_name = function_call_fb.function_name();
        let function_call_type = match function_call_fb.function_call_type() {
            FbFunctionCallType::guest => FunctionCallType::Guest,
            FbFunctionCallType::host => FunctionCallType::Host,
            other => {
                bail!("Invalid function call type: {:?}", other);
            }
        };
        let expected_return_type = function_call_fb.expected_return_type().try_into()?;

        let parameters = function_call_fb
            .parameters()
            .map(|v| {
                v.iter()
                    .map(|p| p.try_into())
                    .collect::<Result<Vec<ParameterValue>>>()
            })
            .transpose()?;

        Ok(Self {
            function_name: function_name.to_string(),
            parameters,
            function_call_type,
            expected_return_type,
        })
    }
}

impl TryFrom<FunctionCall> for Vec<u8> {
    type Error = Error;
    #[cfg_attr(feature = "tracing", instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace"))]
    fn try_from(value: FunctionCall) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let function_name = builder.create_string(&value.function_name);

        let function_call_type = match value.function_call_type {
            FunctionCallType::Guest => FbFunctionCallType::guest,
            FunctionCallType::Host => FbFunctionCallType::host,
        };

        let expected_return_type = value.expected_return_type.into();

        let parameters = match &value.parameters {
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
                        ParameterValue::UInt(ui) => {
                            let hluint = hluint::create(&mut builder, &hluintArgs { value: *ui });
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: FbParameterValue::hluint,
                                    value: Some(hluint.as_union_value()),
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
                        ParameterValue::ULong(ul) => {
                            let hlulong =
                                hlulong::create(&mut builder, &hlulongArgs { value: *ul });
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: FbParameterValue::hlulong,
                                    value: Some(hlulong.as_union_value()),
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
            None => Vec::new(),
        };

        let parameters = if !parameters.is_empty() {
            Some(builder.create_vector(&parameters))
        } else {
            None
        };

        let function_call = FbFunctionCall::create(
            &mut builder,
            &FbFunctionCallArgs {
                function_name: Some(function_name),
                parameters,
                function_call_type,
                expected_return_type,
            },
        );
        builder.finish_size_prefixed(function_call, None);
        let res = builder.finished_data().to_vec();

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use hyperlight_testing::{get_guest_function_call_test_data, get_host_function_call_test_data};

    use super::*;
    use crate::flatbuffer_wrappers::function_types::ReturnType;

    #[test]
    fn read_from_flatbuffer() -> Result<()> {
        let test_data = get_guest_function_call_test_data();
        let function_call = FunctionCall::try_from(test_data.as_slice())?;
        assert_eq!(function_call.function_name, "PrintNineArgs");
        assert!(function_call.parameters.is_some());
        let parameters = function_call.parameters.unwrap();
        assert_eq!(parameters.len(), 9);
        let expected_parameters = vec![
            ParameterValue::String(String::from("Test9")),
            ParameterValue::Int(8),
            ParameterValue::Long(9),
            ParameterValue::String(String::from("Tested")),
            ParameterValue::String(String::from("Test9")),
            ParameterValue::Bool(false),
            ParameterValue::Bool(true),
            ParameterValue::UInt(10),
            ParameterValue::ULong(11),
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
            ParameterValue::String(String::from("Test9")),
            ParameterValue::Int(8),
            ParameterValue::Long(9),
            ParameterValue::String(String::from("Tested")),
            ParameterValue::String(String::from("Test9")),
            ParameterValue::Bool(false),
            ParameterValue::Bool(true),
            ParameterValue::UInt(10),
            ParameterValue::ULong(11),
        ]);
        let guest_function_call = FunctionCall {
            function_name: "PrintNineArgs".to_string(),
            parameters: guest_parameters,
            function_call_type: FunctionCallType::Guest,
            expected_return_type: ReturnType::Int,
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
            expected_return_type: ReturnType::Int,
        };
        let host_function_call_buffer: Vec<u8> = function_call.try_into()?;
        assert_eq!(
            host_function_call_buffer,
            get_host_function_call_test_data()
        );

        Ok(())
    }
}
