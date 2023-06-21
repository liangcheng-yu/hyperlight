use crate::flatbuffers::hyperlight::generated::{
    GuestFunctionDefinition as FbGuestFunctionDefinition,
    GuestFunctionDefinitionArgs as FbGuestFunctionDefinitionArgs, ParameterType as FbParameterType,
};
use crate::guest::function_types::{ParamType, ReturnType};
use crate::mem::ptr::GuestPtr;
use anyhow::Result;
use flatbuffers::{FlatBufferBuilder, WIPOffset};

/// The definition of a function exposed from the guest to the host
#[readonly::make]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestFunctionDefinition {
    /// The function name
    pub function_name: String,
    /// The type of the parameter values for the host function call.
    pub parameter_types: Vec<ParamType>,
    /// The type of the return value from the host function call
    pub return_type: ReturnType,
    /// The function pointer to the guest function
    pub function_pointer: GuestPtr,
}

impl GuestFunctionDefinition {
    /// Create a new `GuestFunctionDetails`.
    pub fn new(
        function_name: String,
        parameter_types: Vec<ParamType>,
        return_type: ReturnType,
        function_pointer: GuestPtr,
    ) -> Self {
        Self {
            function_name,
            parameter_types,
            return_type,
            function_pointer,
        }
    }

    /// Convert this `GuestFunctionDefinition` into a `WIPOffset<FbGuestFunctionDefinition>`.
    pub fn convert_to_flatbuffer_def<'a>(
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
        let function_pointer = self.function_pointer.clone();

        let fb_guest_function_definition: WIPOffset<FbGuestFunctionDefinition> =
            FbGuestFunctionDefinition::create(
                builder,
                &FbGuestFunctionDefinitionArgs {
                    function_name: Some(guest_function_name),
                    return_type,
                    function_pointer: function_pointer.try_into()?,
                    parameters: Some(guest_parameters),
                },
            );
        Ok(fb_guest_function_definition)
    }
}

impl TryFrom<FbGuestFunctionDefinition<'_>> for GuestFunctionDefinition {
    type Error = anyhow::Error;

    fn try_from(value: FbGuestFunctionDefinition) -> Result<Self> {
        let function_name = value.function_name().to_string();
        let return_type = value.return_type().try_into()?;
        let mut parameter_types: Vec<ParamType> = Vec::new();
        let function_pointer = value.function_pointer();
        for fb_pvt in value.parameters() {
            let pvt = fb_pvt.try_into()?;
            parameter_types.push(pvt);
        }

        Ok(Self {
            function_name,
            parameter_types,
            return_type,
            function_pointer: function_pointer.try_into()?,
        })
    }
}

impl TryFrom<&[u8]> for GuestFunctionDefinition {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        let fb_guest_function_definition = flatbuffers::root::<FbGuestFunctionDefinition>(value)?;
        let guest_function_definition: Self = fb_guest_function_definition.try_into()?;
        Ok(guest_function_definition)
    }
}

impl TryFrom<&GuestFunctionDefinition> for Vec<u8> {
    type Error = anyhow::Error;

    fn try_from(value: &GuestFunctionDefinition) -> Result<Self> {
        let mut builder = FlatBufferBuilder::new();
        let fb_guest_function_definition = value.convert_to_flatbuffer_def(&mut builder)?;
        builder.finish(fb_guest_function_definition, None);
        let bytes = builder.finished_data().to_vec();
        Ok(bytes)
    }
}

impl TryFrom<GuestFunctionDefinition> for Vec<u8> {
    type Error = anyhow::Error;

    fn try_from(value: GuestFunctionDefinition) -> Result<Self> {
        let bytes: Vec<u8> = (&value).try_into()?;
        Ok(bytes)
    }
}
