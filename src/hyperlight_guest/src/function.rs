use crate::gen_flatbuffers::hyperlight::generated::{
    GuestFunctionDefinition as FbGuestFunctionDefinition,
    GuestFunctionDefinitionArgs as FbGuestFunctionDefinitionArgs,
    GuestFunctionDetails as FbGuestFunctionDetails,
    GuestFunctionDetailsArgs as FbGuestFunctionDetailsArgs, ParameterType as FbParameterType,
    ReturnType as FbReturnType,
};
use alloc::{string::String, vec::Vec};
use flatbuffers::{FlatBufferBuilder, WIPOffset};

#[derive(Clone)]
pub struct GuestFunctionDefinition {
    pub function_name: String,
    pub parameter_types: Vec<ParameterType>,
    pub return_type: ReturnType,
    pub function_pointer: i64,
}

impl GuestFunctionDefinition {
    pub fn new(
        function_name: String,
        parameters: Vec<ParameterType>,
        return_type: ReturnType,
        function_pointer: i64,
    ) -> Self {
        Self {
            function_name,
            parameter_types: parameters,
            return_type,
            function_pointer,
        }
    }

    pub(crate) fn convert_to_flatbuffer_def<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<FbGuestFunctionDefinition<'a>> {
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
                    function_pointer: function_pointer,
                    parameters: Some(guest_parameters),
                },
            );
        fb_guest_function_definition
    }
}

#[derive(Clone)]
pub enum ParameterType {
    Int,
    Long,
    String,
    Bool,
    VecBytes,
}

impl From<ParameterType> for FbParameterType {
    fn from(value: ParameterType) -> Self {
        match value {
            ParameterType::Int => FbParameterType::hlint,
            ParameterType::Long => FbParameterType::hllong,
            ParameterType::String => FbParameterType::hlstring,
            ParameterType::Bool => FbParameterType::hlbool,
            ParameterType::VecBytes => FbParameterType::hlvecbytes,
        }
    }
}

#[derive(Clone)]
pub enum ReturnType {
    Int,
    Long,
    String,
    Bool,
    Void,
    VecBytes,
}

impl From<ReturnType> for FbReturnType {
    fn from(value: ReturnType) -> Self {
        match value {
            ReturnType::Int => FbReturnType::hlint,
            ReturnType::Long => FbReturnType::hllong,
            ReturnType::String => FbReturnType::hlstring,
            ReturnType::Bool => FbReturnType::hlbool,
            ReturnType::Void => FbReturnType::hlvoid,
            ReturnType::VecBytes => FbReturnType::hlsizeprefixedbuffer,
        }
    }
}

#[derive(Clone)]
pub struct GuestFunctionDetails {
    pub guest_functions: Vec<GuestFunctionDefinition>,
}

impl GuestFunctionDetails {
    pub fn new() -> Self {
        Self {
            guest_functions: Vec::new(),
        }
    }

    pub fn insert_guest_function(&mut self, gfd: GuestFunctionDefinition) {
        self.guest_functions.push(gfd);
    }

    pub fn sort_guest_functions_by_name(&mut self) {
        self.guest_functions
            .sort_by(|a, b| a.function_name.cmp(&b.function_name));
    }
}

impl From<&GuestFunctionDefinition> for Vec<u8> {
    fn from(gfd: &GuestFunctionDefinition) -> Vec<u8> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let guest_function_definition = gfd.convert_to_flatbuffer_def(&mut builder);
        builder.finish_size_prefixed(guest_function_definition, None);
        builder.finished_data().to_vec()
    }
}

impl From<&mut GuestFunctionDetails> for Vec<u8> {
    fn from(guest_function_details: &mut GuestFunctionDetails) -> Self {
        let mut builder = flatbuffers::FlatBufferBuilder::new();

        let mut guest_function_definitions: Vec<flatbuffers::WIPOffset<FbGuestFunctionDefinition>> =
            Vec::new();

        guest_function_details.sort_guest_functions_by_name();

        for guest_function in guest_function_details.guest_functions.iter() {
            guest_function_definitions.push(guest_function.convert_to_flatbuffer_def(&mut builder));
        }

        let guest_functions = builder.create_vector(&guest_function_definitions);

        let guest_function_details = FbGuestFunctionDetails::create(
            &mut builder,
            &FbGuestFunctionDetailsArgs {
                functions: Some(guest_functions),
            },
        );

        builder.finish_size_prefixed(guest_function_details, None);

        builder.finished_data().to_vec()
    }
}