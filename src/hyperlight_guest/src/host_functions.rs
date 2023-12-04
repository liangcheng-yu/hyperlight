use core::slice::from_raw_parts;

use alloc::{format, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::FunctionCall, guest_error::ErrorCode, host_function_details::HostFunctionDetails, function_types::ParameterType,
};

use crate::{guest_error::set_error, P_PEB};

pub(crate) fn validate_host_function_call(function_call: &FunctionCall) {
    // get host function details
    let host_function_details = get_host_function_details();

    // check if there are any host functions
    if host_function_details.host_functions.is_none() {
        set_error(ErrorCode::GuestError, "No host functions found");
        return;
    }

    // check if function w/ given name exists
    let host_function = if let Some(host_function) =
        host_function_details.find_by_function_name(&function_call.function_name)
    {
        host_function
    } else {
        set_error(
            ErrorCode::GuestError,
            &format!(
                "Host Function Not Found: {}",
                function_call.function_name.clone()
            ),
        );
        return;
    };

    let function_call_fparameters = if let Some(parameters) = function_call.parameters.clone() {
        parameters
    } else {
        if host_function.parameter_types.is_some() {
            set_error(
                ErrorCode::GuestError,
                &format!(
                    "Incorrect parameter count for function: {}",
                    function_call.function_name.clone()
                ),
            );
            return;
        }
        Vec::new()
    };

    let function_call_parameter_types = function_call_fparameters
        .iter()
        .map(|p| p.into())
        .collect::<Vec<ParameterType>>();

    // Verify that the function call has the correct parameter types.
    host_function
        .verify_equal_parameter_types(&function_call_parameter_types)
        .map_err(|e| {
            set_error(
                ErrorCode::GuestError,
                &format!(
                    "Incorrect parameter type for function: {}",
                    function_call.function_name.clone()
                ),
            );
            e
        })
        .unwrap();

}

pub(crate) fn get_host_function_details() -> HostFunctionDetails {
    let peb_ptr = unsafe { P_PEB.unwrap() };

    let host_function_details_buffer =
        unsafe { (*peb_ptr).hostFunctionDefinitions.fbHostFunctionDetails } as *const u8;
    let host_function_details_size =
        unsafe { (*peb_ptr).hostFunctionDefinitions.fbHostFunctionDetailsSize };

    let host_function_details_slice: &[u8] = unsafe {
        from_raw_parts(
            host_function_details_buffer,
            host_function_details_size as usize,
        )
    };

    host_function_details_slice.try_into().unwrap()
}
