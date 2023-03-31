extern crate flatbuffers;
use crate::flatbuffers::hyperlight::generated::{
    hlbool, hlboolArgs, hlint, hlintArgs, hllong, hllongArgs, hlstring, hlstringArgs, hlvecbytes,
    hlvecbytesArgs, size_prefixed_root_as_function_call, FunctionCall, FunctionCallArgs, Parameter,
    ParameterArgs, ParameterValue,
};
use crate::mem::layout::SandboxMemoryLayout;
use crate::mem::shared_mem::SharedMemory;
use anyhow::{anyhow, Result};
use flatbuffers::WIPOffset;
use readonly;
use std::convert::{TryFrom, TryInto};

/// `GuestFunctioncall` represents a call to a function in the guest.
#[readonly::make]
#[derive(Debug, Default, Clone)]
pub struct GuestFunctionCall {
    /// The function name
    pub function_name: String,
    /// The parameters for the function call.
    pub parameters: Option<Vec<Param>>,
}

/// This is the type and value of a parameter that can be passed to a guest function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Param {
    /// Parameter is a signed 32 bit integer.
    Int(i32),
    /// Parameter is a signed 64 bit integer.
    Long(i64),
    /// Parameter is a boolean.
    Boolean(bool),
    /// Parameter is a string.
    String(Option<String>),
    /// Parameter is a vector of bytes.
    VecBytes(Option<Vec<u8>>),
}

impl GuestFunctionCall {
    /// Create a new `GuestFunctionCall`.
    pub fn new(function_name: String, parameters: Option<Vec<Param>>) -> Self {
        Self {
            function_name,
            parameters,
        }
    }

    /// Write the guest function call to the shared memory.
    pub fn write_to_memory(
        self,
        guest_mem: &mut SharedMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<()> {
        let guest_function_call_buffer: Vec<u8> = self.try_into()?;

        let buffer_size = {
            let size_u64 = guest_mem.read_u64(layout.get_input_data_size_offset())?;
            usize::try_from(size_u64)
                .map_err(|_| anyhow!("could not convert buffer size u64 ({}) to usize", size_u64))
        }?;

        if guest_function_call_buffer.len() > buffer_size {
            return Err(anyhow!(
                "Guest function call buffer is too big for the input data buffer"
            ));
        }

        guest_mem.copy_from_slice(
            guest_function_call_buffer.as_slice(),
            layout.input_data_buffer_offset,
        )?;
        Ok(())
    }
}

impl TryFrom<&[u8]> for GuestFunctionCall {
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        let guest_function_call_fb =
            size_prefixed_root_as_function_call(value).map_err(|e| anyhow!(e))?;
        let function_name = guest_function_call_fb.function_name();
        let parameters = match guest_function_call_fb.parameters() {
            Some(p) => {
                let len = p.len();
                let mut v: Vec<Param> = Vec::with_capacity(len);
                for i in 0..len {
                    let param = p.get(i);
                    let param_type = param.value_type();
                    match param_type {
                        ParameterValue::hlint => {
                            let hlint = param.value_as_hlint().ok_or_else(|| {
                                anyhow!("Failed to get hlint from parameter {}", i)
                            })?;
                            v.push(Param::Int(hlint.value()));
                        }
                        ParameterValue::hllong => {
                            let hllong = param.value_as_hllong().ok_or_else(|| {
                                anyhow!("Failed to get hlong from parameter {}", i)
                            })?;
                            v.push(Param::Long(hllong.value()));
                        }
                        ParameterValue::hlbool => {
                            let hlbool = param.value_as_hlbool().ok_or_else(|| {
                                anyhow!("Failed to get hlbool from parameter {}", i)
                            })?;
                            v.push(Param::Boolean(hlbool.value()));
                        }
                        ParameterValue::hlstring => {
                            let hlstring = param.value_as_hlstring().ok_or_else(|| {
                                anyhow!("Failed to get hlstring from parameter {}", i)
                            })?;

                            v.push(Param::String(hlstring.value().map(str::to_string)));
                        }
                        ParameterValue::hlvecbytes => {
                            let hlvecbytes = param.value_as_hlvecbytes().ok_or_else(|| {
                                anyhow!("Failed to get hlvecbytes from parameter {}", i)
                            })?;
                            match hlvecbytes.value() {
                                Some(val) => {
                                    v.push(Param::VecBytes(Some(val.iter().collect::<Vec<u8>>())))
                                }
                                None => v.push(Param::VecBytes(None)),
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
        })
    }
}

impl TryFrom<&GuestFunctionCall> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: &GuestFunctionCall) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let function_name = builder.create_string(&value.function_name);

        let vec_parameters = match &value.parameters {
            Some(p) => {
                let num_items = p.len();
                let mut parameters: Vec<WIPOffset<Parameter>> = Vec::with_capacity(num_items);

                for param in p {
                    match param {
                        Param::Int(i) => {
                            let hlint = hlint::create(&mut builder, &hlintArgs { value: *i });
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: ParameterValue::hlint,
                                    value: Some(hlint.as_union_value()),
                                },
                            );
                            parameters.push(parameter);
                        }
                        Param::Long(l) => {
                            let hllong = hllong::create(&mut builder, &hllongArgs { value: *l });
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: ParameterValue::hllong,
                                    value: Some(hllong.as_union_value()),
                                },
                            );
                            parameters.push(parameter);
                        }
                        Param::Boolean(b) => {
                            let hlbool = hlbool::create(&mut builder, &hlboolArgs { value: *b });
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: ParameterValue::hlbool,
                                    value: Some(hlbool.as_union_value()),
                                },
                            );
                            parameters.push(parameter);
                        }
                        Param::String(s) => {
                            let hlstring = match &s {
                                Some(s) => {
                                    let val = builder.create_string(s.as_str());
                                    hlstring::create(
                                        &mut builder,
                                        &hlstringArgs { value: Some(val) },
                                    )
                                }
                                None => {
                                    hlstring::create(&mut builder, &hlstringArgs { value: None })
                                }
                            };
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: ParameterValue::hlstring,
                                    value: Some(hlstring.as_union_value()),
                                },
                            );
                            parameters.push(parameter);
                        }
                        Param::VecBytes(v) => {
                            let vec_bytes = match &v {
                                Some(v) => builder.create_vector(v),
                                None => builder.create_vector(&Vec::<u8>::new()),
                            };

                            let hlvecbytes = hlvecbytes::create(
                                &mut builder,
                                &hlvecbytesArgs {
                                    value: Some(vec_bytes),
                                },
                            );
                            let parameter = Parameter::create(
                                &mut builder,
                                &ParameterArgs {
                                    value_type: ParameterValue::hlvecbytes,
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

        let guest_function_call = FunctionCall::create(
            &mut builder,
            &FunctionCallArgs {
                function_name: Some(function_name),
                parameters,
            },
        );
        builder.finish_size_prefixed(guest_function_call, None);
        let res = builder.finished_data().to_vec();

        // This vector may be converted to a raw pointer and returned via the C API and the C API uses the size prefix to determine the capacity and length of the buffer in order to free the memory  , therefore:
        // 1. the capacity of the vector should be the same as the length
        // 2. the capacity of the vector should be the same as the size of the buffer (from the size prefix) + 4 bytes (the size of the size prefix field is not included in the size)

        let length = unsafe { flatbuffers::read_scalar::<i32>(&res[..4]) };
        if res.capacity() != res.len() || res.capacity() != length as usize + 4 {
            anyhow::bail!("The capacity of the vector is for GuestFunctionCall is incorrect");
        }

        Ok(res)
    }
}

impl TryFrom<GuestFunctionCall> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: GuestFunctionCall) -> Result<Vec<u8>> {
        (&value).try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem::config::SandboxMemoryConfiguration;
    use anyhow::Result;
    use hex_literal::hex;

    #[test]
    fn read_from_flatbuffer() -> Result<()> {
        let test_data = get_test_data();
        let function_call = GuestFunctionCall::try_from(test_data.as_slice())?;
        assert_eq!(function_call.function_name, "PrintSevenArgs");
        assert!(function_call.parameters.is_some());
        let parameters = function_call.parameters.unwrap();
        assert_eq!(parameters.len(), 7);
        let expected_parameters = vec![
            Param::String(Some(String::from("Test7"))),
            Param::Int(8),
            Param::Long(9),
            Param::String(Some(String::from("Tested"))),
            Param::String(Some(String::from("Test7"))),
            Param::Boolean(false),
            Param::Boolean(true),
        ];
        assert!(expected_parameters == parameters);
        Ok(())
    }

    #[test]
    fn write_to_flatbuffer() -> Result<()> {
        let parameters = Some(vec![
            Param::String(Some(String::from("Test7"))),
            Param::Int(8),
            Param::Long(9),
            Param::String(Some(String::from("Tested"))),
            Param::String(Some(String::from("Test7"))),
            Param::Boolean(false),
            Param::Boolean(true),
        ]);
        let function_call = GuestFunctionCall::new("PrintSevenArgs".to_string(), parameters);
        let function_call_buffer: Vec<u8> = function_call.try_into()?;
        assert_eq!(function_call_buffer, get_test_data());
        Ok(())
    }

    #[test]
    fn write_to_memory() -> Result<()> {
        let test_data = get_test_data();
        let function_call = GuestFunctionCall::try_from(test_data.as_slice())?;
        let memory_config = SandboxMemoryConfiguration::new(0, 0, 0, 0, 0, None, None);
        let mut shared_memory = SharedMemory::new(32768)?;
        let memory_layout = SandboxMemoryLayout::new(memory_config, 4096, 4096, 4096)?;
        let result = function_call.write_to_memory(&mut shared_memory, &memory_layout);
        assert!(result.is_err());
        assert_eq!(
            "Guest function call buffer is too big for the input data buffer",
            result.err().unwrap().to_string()
        );

        let function_call = GuestFunctionCall::try_from(test_data.as_slice())?;
        let memory_config = SandboxMemoryConfiguration::new(1024, 0, 0, 0, 0, None, None);
        let memory_layout = SandboxMemoryLayout::new(memory_config, 4096, 4096, 4096)?;
        let mem_size = memory_layout.get_memory_size()?;
        let mut shared_memory = SharedMemory::new(mem_size)?;
        let offset = shared_memory.base_addr();
        memory_layout.write(&mut shared_memory, offset, mem_size)?;

        let result = function_call.write_to_memory(&mut shared_memory, &memory_layout);
        assert!(result.is_ok());

        Ok(())
    }

    // The test data is a valid flatbuffers buffer representing a guestfunction call as follows:
    // int PrintSevenArgs(string="Test7", int=8, long=9, string="Tested", string="Test7", bool=false, bool=true)
    fn get_test_data() -> Vec<u8> {
        hex!("2c010000100000000000000008000c000400080008000000040100000400000007000000d0000000b000000084000000600000003c000000240000000400000054ffffff000000040c000000000006000800070006000000000000018cffffff00000004080000000400040004000000a0ffffff00000003040000007affffff04000000050000005465737437000000c0ffffff00000003040000009affffff04000000060000005465737465640000c4ffffff000000020c000000000006000c00040006000000090000000000000008000c0007000800080000000000000104000000e2ffffff0800000008000e000700080008000000000000030c000000000006000800040006000000040000000500000054657374370000000e0000005072696e74536576656e417267730000").to_vec()
    }
}
