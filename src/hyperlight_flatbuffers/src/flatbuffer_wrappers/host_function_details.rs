use anyhow::{Error, Result, bail};
use flatbuffers::WIPOffset;

use crate::flatbuffers::hyperlight::generated::{
    size_prefixed_root_as_host_function_details,
    HostFunctionDefinition as FbHostFunctionDefinition,
    HostFunctionDetails as FbHostFunctionDetails,
    HostFunctionDetailsArgs as FbHostFunctionDetailsArgs,
};

use super::host_function_definition::HostFunctionDefinition;

/// `HostFunctionDetails` represents the set of functions that the host exposes to the guest.
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

    /// Insert a host function into the host function details.
    pub fn insert_host_function(&mut self, host_function: HostFunctionDefinition) {
        match &mut self.host_functions {
            Some(host_functions) => host_functions.push(host_function),
            None => {
                let host_functions = vec![host_function];
                self.host_functions = Some(host_functions);
            }
        }
    }

    /// Sort the host functions by name.
    pub fn sort_host_functions_by_name(&mut self) {
        match &mut self.host_functions {
            Some(host_functions) => {
                host_functions.sort_by(|a, b| a.function_name.cmp(&b.function_name))
            }
            None => {}
        }
    }
}

impl TryFrom<&[u8]> for HostFunctionDetails {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        let host_function_details_fb = size_prefixed_root_as_host_function_details(value)?;

        let host_function_definitions = match host_function_details_fb.functions() {
            Some(hfd) => {
                let len = hfd.len();
                let mut vec_hfd: Vec<HostFunctionDefinition> = Vec::with_capacity(len);
                for i in 0..len {
                    let fb_host_function_definition = hfd.get(i);
                    let hfdef = HostFunctionDefinition::try_from(&fb_host_function_definition)?;
                    vec_hfd.push(hfdef);
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
    type Error = Error;
    fn try_from(value: &HostFunctionDetails) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let vec_host_function_definitions = match &value.host_functions {
            Some(vec_hfd) => {
                let num_items = vec_hfd.len();
                let mut host_function_definitions: Vec<WIPOffset<FbHostFunctionDefinition>> =
                    Vec::with_capacity(num_items);

                for hfd in vec_hfd {
                    let host_function_definition = hfd.convert_to_flatbuffer_def(&mut builder)?;
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
            bail!("The capacity of the vector is for HostFunctionDetails is incorrect");
        }

        Ok(res)
    }
}
