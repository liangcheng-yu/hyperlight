use alloc::{string::{String, ToString}, vec::Vec};
use anyhow::{anyhow, Error, Result};
use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::flatbuffers::hyperlight::generated::{
    GuestFunctionDefinition as FbGuestFunctionDefinition,
    GuestFunctionDefinitionArgs as FbGuestFunctionDefinitionArgs, ParameterType as FbParameterType,
};

use super::function_types::{ParameterType, ReturnType};

/// The definition of a function exposed from the guest to the host
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestFunctionDefinition {
    /// The function name
    pub function_name: String,
    /// The type of the parameter values for the host function call.
    pub parameter_types: Vec<ParameterType>,
    /// The type of the return value from the host function call
    pub return_type: ReturnType,
    /// The function pointer to the guest function
    pub function_pointer: i64,
}

impl GuestFunctionDefinition {
    /// Create a new `GuestFunctionDefinition`.
    pub fn new(
        function_name: String,
        parameter_types: Vec<ParameterType>,
        return_type: ReturnType,
        function_pointer: i64,
    ) -> Self {
        Self {
            function_name,
            parameter_types,
            return_type,
            function_pointer,
        }
    }

    /// Convert this `GuestFunctionDefinition` into a `WIPOffset<FbGuestFunctionDefinition>`.
    pub(crate) fn convert_to_flatbuffer_def<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> Result<WIPOffset<FbGuestFunctionDefinition<'a>>> {
        let guest_function_name = builder.create_string(&self.function_name);
        let return_type = self.return_type.clone().into();
        let guest_parameters = {
            let num_items = self.parameter_types.len();
            let mut vec_parameters: Vec<FbParameterType> = Vec::with_capacity(num_items);
            for pvt in &self.parameter_types {
                let fb_pvt = pvt.clone().into();
                vec_parameters.push(fb_pvt);
            }
            builder.create_vector(&vec_parameters)
        };
        let function_pointer = self.function_pointer;

        let fb_guest_function_definition: WIPOffset<FbGuestFunctionDefinition> =
            FbGuestFunctionDefinition::create(
                builder,
                &FbGuestFunctionDefinitionArgs {
                    function_name: Some(guest_function_name),
                    return_type,
                    function_pointer,
                    parameters: Some(guest_parameters),
                },
            );
        Ok(fb_guest_function_definition)
    }
}

impl TryFrom<FbGuestFunctionDefinition<'_>> for GuestFunctionDefinition {
    type Error = Error;

    fn try_from(value: FbGuestFunctionDefinition) -> Result<Self> {
        let function_name = value.function_name().to_string();
        let return_type = value.return_type().try_into().map_err(|_| {
            anyhow!(
                "Failed to convert return type for function {}",
                function_name
            )
        })?;
        let mut parameter_types: Vec<ParameterType> = Vec::new();
        let function_pointer = value.function_pointer();
        for fb_pvt in value.parameters() {
            let pvt = fb_pvt.try_into().map_err(|_| {
                anyhow!(
                    "Failed to convert parameter type for function {}",
                    function_name
                )
            })?;
            parameter_types.push(pvt);
        }

        Ok(Self {
            function_name,
            parameter_types,
            return_type,
            function_pointer,
        })
    }
}

impl TryFrom<&[u8]> for GuestFunctionDefinition {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        let fb_guest_function_definition =
            flatbuffers::root::<FbGuestFunctionDefinition>(value).unwrap();
        let guest_function_definition: Self = fb_guest_function_definition.try_into()?;
        Ok(guest_function_definition)
    }
}

impl TryFrom<&GuestFunctionDefinition> for Vec<u8> {
    type Error = Error;

    fn try_from(value: &GuestFunctionDefinition) -> Result<Self> {
        let mut builder = FlatBufferBuilder::new();
        let fb_guest_function_definition = value.convert_to_flatbuffer_def(&mut builder)?;
        builder.finish(fb_guest_function_definition, None);
        let bytes = builder.finished_data().to_vec();
        Ok(bytes)
    }
}
