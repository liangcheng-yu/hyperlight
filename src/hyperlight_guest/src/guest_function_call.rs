use core::{ptr::copy_nonoverlapping, slice::from_raw_parts};

use alloc::{string::ToString, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::{FunctionCall, FunctionCallType},
    function_types::ParameterType,
    guest_error::ErrorCode,
    guest_function_details::GuestFunctionDetails,
};

use crate::{
    entrypoint::halt,
    guest_error::{reset_error, set_error},
    GUEST_FUNCTIONS, P_PEB,
};

type GuestFunc = fn(&FunctionCall) -> Vec<u8>;
pub(crate) fn call_guest_function(function_call: &FunctionCall) -> Vec<u8> {
    let function_call_fparameters = function_call.parameters.clone().unwrap_or_default();
    let function_call_fname = function_call.clone().function_name;

    // Verify that the function does not have more than 10 parameters.
    const MAX_PARAMETERS: usize = 10;
    assert!(
        function_call_fparameters.len() <= MAX_PARAMETERS,
        "Exceeded maximum parameter count"
    );

    // Get registered function definitions.
    let guest_function_details: GuestFunctionDetails =
        unsafe { GUEST_FUNCTIONS.as_slice().try_into().unwrap() };

    // Find the function definition for the function call.
    if let Some(registered_function_definition) =
        guest_function_details.find_by_function_name(&function_call_fname)
    {
        // Verify that the function call has the correct number of parameters.
        assert!(
            function_call_fparameters.len() == registered_function_definition.parameter_types.len(),
            "Incorrect parameter count"
        );

        let function_call_parameter_types = function_call_fparameters
            .iter()
            .map(|p| p.into())
            .collect::<Vec<ParameterType>>();

        let p_function = unsafe {
            let function_pointer = registered_function_definition.function_pointer;
            core::mem::transmute::<i64, GuestFunc>(function_pointer)
        };

        // Verify that the function call has the correct parameter types.
        registered_function_definition
            .verify_equal_parameter_types(&function_call_parameter_types)
            .unwrap();

        // If a parameter is a vector of bytes (hlvecbytes), then we expect the next parameter
        // to be an integer specifying the length of that vector.
        // If this integer is not present, we should return an error.
        registered_function_definition
            .verify_vector_parameter_lengths(function_call_parameter_types)
            .map_err(|e| set_error(ErrorCode::ArrayLengthParamIsMissing, &e.to_string()))
            .unwrap();

        p_function(function_call)
    } else {
        extern "C" {
            #[allow(improper_ctypes)]
            fn guest_dispatch_function(function_call: &FunctionCall) -> Vec<u8>;
        }

        // If the function was not found call the guest_dispatch_function method.
        unsafe { guest_dispatch_function(function_call) }
    }
}

pub(crate) fn dispatch_function() {
    reset_error();

    let peb_ptr = unsafe { P_PEB.unwrap() };

    let idb = unsafe {
        from_raw_parts(
            (*peb_ptr).inputdata.inputDataBuffer as *mut u8,
            (*peb_ptr).inputdata.inputDataSize as usize,
        )
    };
    let function_call = FunctionCall::try_from(idb).unwrap();

    // Validate this is a Guest Function Call
    if function_call.function_call_type() != FunctionCallType::Guest {
        set_error(ErrorCode::GuestError, "Invalid Function Call Type");
        return;
    }

    let result_vec = call_guest_function(&function_call);

    unsafe {
        let output_data_buffer = (*peb_ptr).outputdata.outputDataBuffer as *mut u8;
        let size_with_prefix = result_vec.len();

        copy_nonoverlapping(result_vec.as_ptr(), output_data_buffer, size_with_prefix);
    }
    halt();
}
