use core::{ptr::copy_nonoverlapping, slice::from_raw_parts};

use alloc::{format, string::ToString, vec::Vec};
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
pub(crate) fn call_guest_function(function_call: &FunctionCall) -> Result<Vec<u8>, ()> {
    let function_call_fparameters = function_call.parameters.clone().unwrap_or_default();
    let function_call_fname = function_call.clone().function_name;

    // Verify that the function does not have more than 10 parameters.
    const MAX_PARAMETERS: usize = 10;
    if function_call_fparameters.len() > MAX_PARAMETERS {
        set_error(ErrorCode::GuestError, "Too many parameters");
        return Err(());
    }

    // Get registered function definitions.
    let guest_function_details: GuestFunctionDetails =
        unsafe { GUEST_FUNCTIONS.as_slice().try_into().unwrap() };

    // Find the function definition for the function call.
    if let Some(registered_function_definition) =
        guest_function_details.find_by_function_name(&function_call_fname)
    {
        // Verify that the function call has the correct number of parameters.
        if function_call_fparameters.len() != registered_function_definition.parameter_types.len() {
            set_error(
                ErrorCode::GuestFunctionIncorrecNoOfParameters,
                format!(
                    "Called function {} with {} parameters but it takes {}.",
                    function_call_fname,
                    function_call_fparameters.len(),
                    registered_function_definition.parameter_types.len()
                )
                .as_str(),
            );

            return Err(());
        }

        let function_call_parameter_types = function_call_fparameters
            .iter()
            .map(|p| p.into())
            .collect::<Vec<ParameterType>>();

        let p_function = unsafe {
            let function_pointer = registered_function_definition.function_pointer;
            core::mem::transmute::<i64, GuestFunc>(function_pointer)
        };

        // Verify that the function call has the correct parameter types.
        if let Err(i) = registered_function_definition
            .verify_equal_parameter_types(&function_call_parameter_types)
        {
            set_error(
                ErrorCode::GuestFunctionParameterTypeMismatch,
                format!("Function {} parameter {}.", function_call_fname, i).as_str(),
            );
            return Err(());
        }

        // If a parameter is a vector of bytes (hlvecbytes), then we expect the next parameter
        // to be an integer specifying the length of that vector.
        // If this integer is not present, we should return an error.
        if let Err(e) = registered_function_definition
            .verify_vector_parameter_lengths(function_call_parameter_types)
        {
            set_error(ErrorCode::ArrayLengthParamIsMissing, &e.to_string());
            return Err(());
        }

        Ok(p_function(function_call))
    } else {
        // If the function was not found call the guest_dispatch_function method.

        // TODO: ideally we would define a default implementation of this with weak linkage so the guest is not required
        // to implement the function but its seems that weak linkage is an unstable feature so for now its probably better
        // to not do that.
        extern "Rust" {
            fn guest_dispatch_function(function_call: &FunctionCall) -> Vec<u8>;
        }

        unsafe { Ok(guest_dispatch_function(function_call)) }
    }
}

// This function is marked as no_mangle/inline to prevent the compiler from inlining it , if its inlined the epilogue will not be called
// and we will leak memory as the epilogue will not be called as halt() is not going to return.
#[no_mangle]
#[inline(never)]
fn internal_dispatch_function() {
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

    let result_vec = match call_guest_function(&function_call) {
        Ok(v) => v,
        Err(_) => return,
    };

    unsafe {
        let output_data_buffer = (*peb_ptr).outputdata.outputDataBuffer as *mut u8;

        copy_nonoverlapping(result_vec.as_ptr(), output_data_buffer, result_vec.len());
    }
}

// This is implemented as a separate function to make sure that epilogue in the internal_dispatch_function is called before the halt()
// which if it were included in the internal_dispatch_function cause the epilogue to not be called because the halt() would not return
// when running in the hypervisor.
pub(crate) fn dispatch_function() {
    internal_dispatch_function();
    halt();
}
