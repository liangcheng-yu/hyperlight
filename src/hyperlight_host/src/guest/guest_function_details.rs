use anyhow::Result;

use crate::flatbuffers::hyperlight::generated::size_prefixed_root_as_guest_function_details;

use super::guest_function_definition::GuestFunctionDefinition;

/// Represents the functions that the guest exposes to the host.
#[readonly::make]
#[derive(Debug, Default, Clone)]
pub struct GuestFunctionDetails {
    /// The guest functions
    pub guest_functions: Vec<GuestFunctionDefinition>,
}

impl GuestFunctionDetails {
    /// Create a new `GuestFunctionDetails`.
    pub fn new(guest_functions: Vec<GuestFunctionDefinition>) -> Self {
        Self { guest_functions }
    }
}

impl TryFrom<&[u8]> for GuestFunctionDetails {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let guest_function_details_fb = size_prefixed_root_as_guest_function_details(bytes)?;

        let guest_function_definitions = {
            let mut guest_function_definitions: Vec<GuestFunctionDefinition> = Vec::new();
            for guest_function in guest_function_details_fb.functions().iter() {
                let guest_function_definition = GuestFunctionDefinition::try_from(guest_function)?;
                guest_function_definitions.push(guest_function_definition);
            }
            guest_function_definitions
        };

        Ok(Self::new(guest_function_definitions))
    }
}

impl TryFrom<&GuestFunctionDetails> for Vec<u8> {
    type Error = anyhow::Error;

    fn try_from(guest_function_details: &GuestFunctionDetails) -> Result<Self> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();

        let mut guest_function_definitions: Vec<
            flatbuffers::WIPOffset<
                crate::flatbuffers::hyperlight::generated::GuestFunctionDefinition,
            >,
        > = Vec::new();
        for guest_function in guest_function_details.guest_functions.iter() {
            guest_function_definitions
                .push(guest_function.convert_to_flatbuffer_def(&mut builder)?);
        }

        let guest_functions = builder.create_vector(&guest_function_definitions);

        let guest_function_details =
            crate::flatbuffers::hyperlight::generated::GuestFunctionDetails::create(
                &mut builder,
                &crate::flatbuffers::hyperlight::generated::GuestFunctionDetailsArgs {
                    functions: Some(guest_functions),
                },
            );

        builder.finish_size_prefixed(guest_function_details, None);

        Ok(builder.finished_data().to_vec())
    }
}
