use alloc::{string::ToString, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_types::{ParameterType, ReturnType},
    guest_function_definition::GuestFunctionDefinition,
};

use crate::{GUEST_FUNCTIONS, GUEST_FUNCTIONS_BUILDER};

pub fn create_function_definition(
    function_name: &str,
    p_function: i64,
    parameters: &[ParameterType],
) -> GuestFunctionDefinition {
    GuestFunctionDefinition::new(
        function_name.to_string(),
        parameters.to_vec(),
        ReturnType::Int, // Currently, all functions return an int
        p_function,
    )
}

pub fn register_function(function_definition: GuestFunctionDefinition) {
    unsafe {
        let gfd = &mut GUEST_FUNCTIONS_BUILDER;
        gfd.push(function_definition);
    }
}

pub fn finalise_function_table() {
    unsafe {
        let gfd = &mut GUEST_FUNCTIONS_BUILDER;
        gfd.sort_by_function_name();

        let gfd_finalised: Vec<u8> = Vec::try_from(&*gfd).unwrap();
        GUEST_FUNCTIONS = gfd_finalised.clone();
    }
}
