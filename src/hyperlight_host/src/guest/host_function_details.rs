extern crate flatbuffers;
use super::host_function_definition::{HostFunctionDefinition, ParamValueType, ReturnValueType};
use crate::flatbuffers::hyperlight::generated::{
    size_prefixed_root_as_host_function_details,
    HostFunctionDefinition as FbHostFunctionDefinition,
    HostFunctionDefinitionArgs as FbHostFunctionDefinitionArgs,
    HostFunctionDetails as FbHostFunctionDetails,
    HostFunctionDetailsArgs as FbHostFunctionDetailsArgs, ParameterType, ReturnType,
};
use crate::mem::layout::SandboxMemoryLayout;
use crate::mem::shared_mem::SharedMemory;
use anyhow::{anyhow, bail, Result};
use flatbuffers::WIPOffset;
use readonly;
use std::convert::{TryFrom, TryInto};

/// `HostFunctionDetails` represents the set of functions that the host exposes to the guest.
#[readonly::make]
#[derive(Debug, Default, Clone)]
pub struct HostFunctionDetails {
    /// The host functions.
    pub host_functions: Option<Vec<HostFunctionDefinition>>,
}

impl HostFunctionDetails {
    /// Create a new `HostFunctionDetails`.
    pub fn new(host_functions: Option<Vec<HostFunctionDefinition>>) -> Self {
        Self { host_functions }
    }

    /// Write the host function details to the shared memory.
    pub fn write_to_memory(
        self,
        guest_mem: &mut SharedMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<()> {
        let host_function_call_buffer: Vec<u8> = self.try_into()?;

        let buffer_size = {
            let size_u64 =
                guest_mem.read_u64(layout.get_host_function_definitions_size_offset())?;
            usize::try_from(size_u64)
                .map_err(|_| anyhow!("could not convert buffer size u64 ({}) to usize", size_u64))
        }?;

        if host_function_call_buffer.len() > buffer_size {
            bail!(
                "Host Function Details buffer is too big for the host_function_definitions buffer"
            );
        }

        guest_mem.copy_from_slice(
            host_function_call_buffer.as_slice(),
            layout.host_function_definitions_offset,
        )?;
        Ok(())
    }
}

impl TryFrom<&[u8]> for HostFunctionDetails {
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        let host_function_details_fb =
            size_prefixed_root_as_host_function_details(value).map_err(|e| anyhow!(e))?;

        let host_function_definitions = match host_function_details_fb.functions() {
            Some(hfd) => {
                let len = hfd.len();
                let mut vec_hfd: Vec<HostFunctionDefinition> = Vec::with_capacity(len);
                for i in 0..len {
                    let fb_host_function_definition = hfd.get(i);
                    let host_function_name = fb_host_function_definition.function_name();
                    let return_value_type = match fb_host_function_definition.return_type() {
                        ReturnType::hlint => ReturnValueType::Int,
                        ReturnType::hllong => ReturnValueType::Long,
                        ReturnType::hlstring => ReturnValueType::String,
                        ReturnType::hlbool => ReturnValueType::Boolean,
                        ReturnType::hlvoid => ReturnValueType::Void,
                        _ => {
                            bail!(
                                "Unknown return type: {:?}",
                                fb_host_function_definition.return_type()
                            )
                        }
                    };
                    let param_value_types = match fb_host_function_definition.parameters() {
                        Some(pvt) => {
                            let len = pvt.len();
                            let mut pv: Vec<ParamValueType> = Vec::with_capacity(len);
                            for i in 0..len {
                                let param_type = pvt.get(i);
                                match param_type {
                                    ParameterType::hlint => {
                                        pv.push(ParamValueType::Int);
                                    }
                                    ParameterType::hllong => {
                                        pv.push(ParamValueType::Long);
                                    }
                                    ParameterType::hlbool => {
                                        pv.push(ParamValueType::Boolean);
                                    }
                                    ParameterType::hlstring => {
                                        pv.push(ParamValueType::String);
                                    }
                                    ParameterType::hlvecbytes => {
                                        pv.push(ParamValueType::VecBytes);
                                    }
                                    _ => {
                                        bail!("Unknown parameter type: {:?}", param_type)
                                    }
                                };
                            }
                            Some(pv)
                        }
                        None => None,
                    };
                    vec_hfd.push(HostFunctionDefinition::new(
                        host_function_name.to_string(),
                        param_value_types,
                        return_value_type,
                    ));
                }

                Some(vec_hfd)
            }

            None => None,
        };

        Ok(Self {
            host_functions: host_function_definitions,
        })
    }
}

impl TryFrom<&HostFunctionDetails> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: &HostFunctionDetails) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let vec_host_function_definitions = match &value.host_functions {
            Some(vec_hfd) => {
                let num_items = vec_hfd.len();
                let mut host_function_definitions: Vec<WIPOffset<FbHostFunctionDefinition>> =
                    Vec::with_capacity(num_items);

                for hfd in vec_hfd {
                    let host_function_name = builder.create_string(&hfd.function_name);
                    let return_value_type = match &hfd.return_type {
                        ReturnValueType::Int => ReturnType::hlint,
                        ReturnValueType::Long => ReturnType::hllong,
                        ReturnValueType::String => ReturnType::hlstring,
                        ReturnValueType::Boolean => ReturnType::hlbool,
                        ReturnValueType::Void => ReturnType::hlvoid,
                    };
                    let vec_parameters = match &hfd.parameter_types {
                        Some(vec_pvt) => {
                            let num_items = vec_pvt.len();
                            let mut parameters: Vec<ParameterType> = Vec::with_capacity(num_items);
                            for pvt in vec_pvt {
                                match pvt {
                                    ParamValueType::Int => {
                                        parameters.push(ParameterType::hlint);
                                    }
                                    ParamValueType::Long => {
                                        parameters.push(ParameterType::hllong);
                                    }
                                    ParamValueType::Boolean => {
                                        parameters.push(ParameterType::hlbool);
                                    }
                                    ParamValueType::String => {
                                        parameters.push(ParameterType::hlstring);
                                    }
                                    ParamValueType::VecBytes => {
                                        parameters.push(ParameterType::hlvecbytes);
                                    }
                                };
                            }
                            Some(builder.create_vector(&parameters))
                        }
                        None => None,
                    };

                    let host_function_definition = FbHostFunctionDefinition::create(
                        &mut builder,
                        &FbHostFunctionDefinitionArgs {
                            function_name: Some(host_function_name),
                            return_type: return_value_type,
                            parameters: vec_parameters,
                        },
                    );
                    host_function_definitions.push(host_function_definition);
                }

                Some(host_function_definitions)
            }
            None => None,
        };

        let fb_host_function_definitions =
            vec_host_function_definitions.map(|v| builder.create_vector(&v));

        let host_function_details = FbHostFunctionDetails::create(
            &mut builder,
            &FbHostFunctionDetailsArgs {
                functions: fb_host_function_definitions,
            },
        );
        builder.finish_size_prefixed(host_function_details, None);
        let res = builder.finished_data().to_vec();

        // This vector may be converted to a raw pointer and returned via the C API and the C API uses the size prefix to determine the capacity and length of the buffer in order to free the memory  , therefore:
        // 1. the capacity of the vector should be the same as the length
        // 2. the capacity of the vector should be the same as the size of the buffer (from the size prefix) + 4 bytes (the size of the size prefix field is not included in the size)

        let length = unsafe { flatbuffers::read_scalar::<i32>(&res[..4]) };
        if res.capacity() != res.len() || res.capacity() != length as usize + 4 {
            anyhow::bail!("The capacity of the vector is for HostFunctionDetails is incorrect");
        }

        Ok(res)
    }
}

impl TryFrom<HostFunctionDetails> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: HostFunctionDetails) -> Result<Vec<u8>> {
        (&value).try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem::config::SandboxMemoryConfiguration;
    use anyhow::{Ok, Result};
    use hex_literal::hex;

    #[test]
    fn read_from_flatbuffer() -> Result<()> {
        let (test_data, test_host_function_definitions) = get_test_data();
        let host_function_details = HostFunctionDetails::try_from(test_data.as_slice())?;
        assert!(host_function_details.host_functions.is_some());
        let host_function_definitions = host_function_details.host_functions.unwrap();
        assert_eq!(host_function_definitions.len(), 7);
        assert!(test_host_function_definitions == host_function_definitions);
        Ok(())
    }
    #[test]
    fn write_to_flatbuffer() -> Result<()> {
        let (test_data, test_host_function_definitions) = get_test_data();
        let host_function_details = HostFunctionDetails::new(Some(test_host_function_definitions));
        let flatbuffer = Vec::<u8>::try_from(&host_function_details)?;
        assert_eq!(flatbuffer, test_data);
        Ok(())
    }
    #[test]
    fn write_to_memory() -> Result<()> {
        let (test_data, _) = get_test_data();
        let host_function_details = HostFunctionDetails::try_from(test_data.as_slice())?;
        let memory_config = SandboxMemoryConfiguration::new(0, 0, 0, 0, 0);
        let mut shared_memory = SharedMemory::new(32768)?;
        let memory_layout = SandboxMemoryLayout::new(memory_config, 4096, 4096, 4096)?;
        let result = host_function_details.write_to_memory(&mut shared_memory, &memory_layout);
        assert!(result.is_err());
        assert_eq!(
            "Host Function Details buffer is too big for the host_function_definitions buffer",
            result.err().unwrap().to_string()
        );

        let (test_data, _) = get_test_data();
        let host_function_details = HostFunctionDetails::try_from(test_data.as_slice())?;
        let memory_config = SandboxMemoryConfiguration::new(1024, 0, 0, 0, 0);
        let memory_layout = SandboxMemoryLayout::new(memory_config, 4096, 4096, 4096)?;
        let mem_size = memory_layout.get_memory_size()?;
        let mut shared_memory = SharedMemory::new(mem_size)?;
        let offset = shared_memory.base_addr();
        memory_layout.write(&mut shared_memory, offset, mem_size)?;

        let result = host_function_details.write_to_memory(&mut shared_memory, &memory_layout);
        assert!(result.is_ok());

        Ok(())
    }

    // the vec<u8> returned from this function is a flatbuffer representation of the Vec<HostFunctionDefinitions> with a HostFunctionDetails as the root, these data are equivalent .

    fn get_test_data() -> (Vec<u8>, Vec<HostFunctionDefinition>) {
        let mut host_function_definitions = Vec::<HostFunctionDefinition>::new();

        let host_function_definition =
            HostFunctionDefinition::new(String::from("GetOSPageSize"), None, ReturnValueType::Int);

        host_function_definitions.push(host_function_definition);

        let host_function_definition = HostFunctionDefinition::new(
            String::from("GetStackBoundary"),
            None,
            ReturnValueType::Long,
        );

        host_function_definitions.push(host_function_definition);

        let host_function_definition =
            HostFunctionDefinition::new(String::from("GetTickCount"), None, ReturnValueType::Long);

        host_function_definitions.push(host_function_definition);

        let host_function_definition = HostFunctionDefinition::new(
            String::from("GetTimeSinceBootMicrosecond"),
            None,
            ReturnValueType::Long,
        );

        host_function_definitions.push(host_function_definition);

        let host_function_definition =
            HostFunctionDefinition::new(String::from("GetTwo"), None, ReturnValueType::Int);

        host_function_definitions.push(host_function_definition);

        let host_function_definition = HostFunctionDefinition::new(
            String::from("HostMethod1"),
            Some(vec![ParamValueType::String]),
            ReturnValueType::Int,
        );

        host_function_definitions.push(host_function_definition);

        let host_function_definition = HostFunctionDefinition::new(
            String::from("StaticMethodWithArgs"),
            Some(vec![ParamValueType::String, ParamValueType::Int]),
            ReturnValueType::Int,
        );

        host_function_definitions.push(host_function_definition);

        (hex!("3401000004000000f2feffff040000000700000008010000dc000000b000000080000000680000004000000004000000d0ffffff10000000040000000200000002000000140000005374617469634d6574686f6457697468417267730000000008000c000400080008000000100000000400000001000000020000000b000000486f73744d6574686f64310076ffffff040000000600000047657454776f0000b6ffffff00000001040000001b00000047657454696d6553696e6365426f6f744d6963726f7365636f6e6400e2ffffff00000001040000000c0000004765745469636b436f756e7400000a000c000800000007000a000000000000010400000010000000476574537461636b426f756e64617279000006000800040006000000040000000d0000004765744f535061676553697a65000000").to_vec(), host_function_definitions)
    }
}
