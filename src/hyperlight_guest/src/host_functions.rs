use core::slice::from_raw_parts;

use alloc::{format, string::ToString, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::FunctionCall, function_types::ParameterType, guest_error::ErrorCode,
    host_function_details::HostFunctionDetails,
};

use crate::{
    error::{HyperlightGuestError, Result},
    P_PEB,
};

pub(crate) fn validate_host_function_call(function_call: &FunctionCall) -> Result<()> {
    // get host function details
    let host_function_details = get_host_function_details();

    // check if there are any host functions
    if host_function_details.host_functions.is_none() {
        return Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            "No host functions found".to_string(),
        ));
    }

    // check if function w/ given name exists
    let host_function = if let Some(host_function) =
        host_function_details.find_by_function_name(&function_call.function_name)
    {
        host_function
    } else {
        return Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            format!(
                "Host Function Not Found: {}",
                function_call.function_name.clone()
            ),
        ));
    };

    let function_call_fparameters = if let Some(parameters) = function_call.parameters.clone() {
        parameters
    } else {
        if host_function.parameter_types.is_some() {
            return Err(HyperlightGuestError::new(
                ErrorCode::GuestError,
                format!(
                    "Incorrect parameter count for function: {}",
                    function_call.function_name.clone()
                ),
            ));
        }

        Vec::new() // if no parameters (and no mismatches), return empty vector
    };

    let function_call_parameter_types = function_call_fparameters
        .iter()
        .map(|p| p.into())
        .collect::<Vec<ParameterType>>();

    // Verify that the function call has the correct parameter types.
    host_function
        .verify_equal_parameter_types(&function_call_parameter_types)
        .map_err(|_| {
            HyperlightGuestError::new(
                ErrorCode::GuestError,
                format!(
                    "Incorrect parameter type for function: {}",
                    function_call.function_name.clone()
                ),
            )
        })?;

    Ok(())
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

    host_function_details_slice
        .try_into()
        .expect("Failed to convert buffer to HostFunctionDetails")
}
