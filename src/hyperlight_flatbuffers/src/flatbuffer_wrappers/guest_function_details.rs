use alloc::vec::Vec;
use anyhow::{anyhow, Error, Result};
use tracing::{instrument, Span};

use super::guest_function_definition::GuestFunctionDefinition;
use crate::flatbuffers::hyperlight::generated::{
    size_prefixed_root_as_guest_function_details,
    GuestFunctionDefinition as FbGuestFunctionDefinition,
    GuestFunctionDetails as FbGuestFunctionDetails,
    GuestFunctionDetailsArgs as FbGuestFunctionDetailsArgs,
};

/// Represents the functions that the guest exposes to the host.
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

    /// Insert a new `GuestFunctionDefinition` into the `GuestFunctionDetails`.
    pub fn insert(&mut self, guest_function: GuestFunctionDefinition) {
        self.guest_functions.push(guest_function);
    }

    /// Sort the `GuestFunctionDetails` by the `GuestFunctionDefinition`'s `name` field.
    pub fn sort(&mut self) {
        self.guest_functions
            .sort_by(|a, b| a.function_name.cmp(&b.function_name));
    }
}

impl TryFrom<&[u8]> for GuestFunctionDetails {
    type Error = Error;
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn try_from(bytes: &[u8]) -> Result<Self> {
        let guest_function_details_fb =
            size_prefixed_root_as_guest_function_details(bytes).unwrap();

        let guest_function_definitions = {
            let mut guest_function_definitions: Vec<GuestFunctionDefinition> = Vec::new();
            for guest_function in guest_function_details_fb.functions().iter() {
                let guest_function_definition = GuestFunctionDefinition::try_from(guest_function)
                    .map_err(|e| {
                    anyhow!("Failed to convert guest function definition: {}", e)
                })?;
                guest_function_definitions.push(guest_function_definition);
            }
            guest_function_definitions
        };

        Ok(Self::new(guest_function_definitions))
    }
}

impl TryFrom<&GuestFunctionDetails> for Vec<u8> {
    type Error = Error;
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn try_from(guest_function_details: &GuestFunctionDetails) -> Result<Self> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();

        let mut guest_function_definitions: Vec<flatbuffers::WIPOffset<FbGuestFunctionDefinition>> =
            Vec::new();
        for guest_function in guest_function_details.guest_functions.iter() {
            guest_function_definitions.push(
                guest_function
                    .convert_to_flatbuffer_def(&mut builder)
                    .map_err(|e| anyhow!("Failed to convert guest function definition: {}", e))?,
            );
        }

        let guest_functions = builder.create_vector(&guest_function_definitions);

        let guest_function_details = FbGuestFunctionDetails::create(
            &mut builder,
            &FbGuestFunctionDetailsArgs {
                functions: Some(guest_functions),
            },
        );

        builder.finish_size_prefixed(guest_function_details, None);

        Ok(builder.finished_data().to_vec())
    }
}
