use alloc::{format, string::ToString, vec::Vec};

use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::{FunctionCall, FunctionCallType},
    function_types::ParameterType,
    guest_error::ErrorCode,
    guest_function_details::GuestFunctionDetails,
};

use crate::{
    entrypoint::halt,
    error::{HyperlightGuestError, Result},
    guest_error::{reset_error, set_error},
    shared_input_data::try_pop_shared_input_data_into,
    shared_output_data::push_shared_output_data,
    GUEST_FUNCTIONS,
};

type GuestFunc = fn(&FunctionCall) -> Result<Vec<u8>>;
pub(crate) fn call_guest_function(function_call: &FunctionCall) -> Result<Vec<u8>> {
    // Validate this is a Guest Function Call
    if function_call.function_call_type() != FunctionCallType::Guest {
        return Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            format!(
                "Invalid function call type: {:#?}, should be Guest.",
                function_call.function_call_type()
            ),
        ));
    }

    let function_call_fparameters = function_call.parameters.clone().unwrap_or_default();
    let function_call_fname = function_call.clone().function_name;

    // Verify that the function does not have more than 10 parameters.
    const MAX_PARAMETERS: usize = 10;
    if function_call_fparameters.len() > MAX_PARAMETERS {
        return Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            format!(
                "Function {} has too many parameters: {} (max allowed is 10).",
                function_call_fname,
                function_call_fparameters.len()
            ),
        ));
    }

    // Get registered function definitions.
    let guest_function_details: GuestFunctionDetails = unsafe {
        GUEST_FUNCTIONS
            .as_slice()
            .try_into()
            .expect("Invalid GuestFunctionDetails")
    };

    // Find the function definition for the function call.
    if let Some(registered_function_definition) =
        guest_function_details.find_by_function_name(&function_call_fname)
    {
        // Verify that the function call has the correct number of parameters.
        if function_call_fparameters.len() != registered_function_definition.parameter_types.len() {
            return Err(HyperlightGuestError::new(
                ErrorCode::GuestFunctionIncorrecNoOfParameters,
                format!(
                    "Called function {} with {} parameters but it takes {}.",
                    function_call_fname,
                    function_call_fparameters.len(),
                    registered_function_definition.parameter_types.len()
                ),
            ));
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
            return Err(HyperlightGuestError::new(
                ErrorCode::GuestFunctionParameterTypeMismatch,
                format!("Function {} parameter {}.", function_call_fname, i),
            ));
        }

        // If a parameter is a vector of bytes (hlvecbytes), then we expect the next parameter
        // to be an integer specifying the length of that vector.
        // If this integer is not present, we should return an error.
        if let Err(e) = registered_function_definition
            .verify_vector_parameter_lengths(function_call_parameter_types)
        {
            return Err(HyperlightGuestError::new(
                ErrorCode::ArrayLengthParamIsMissing,
                e.to_string(),
            ));
        }

        p_function(function_call)
    } else {
        // If the function was not found call the guest_dispatch_function method.

        // TODO: ideally we would define a default implementation of this with weak linkage so the guest is not required
        // to implement the function but its seems that weak linkage is an unstable feature so for now its probably better
        // to not do that.
        extern "Rust" {
            fn guest_dispatch_function(function_call: &FunctionCall) -> Result<Vec<u8>>;
        }

        unsafe { guest_dispatch_function(function_call) }
    }
}

// This function is marked as no_mangle/inline to prevent the compiler from inlining it , if its inlined the epilogue will not be called
// and we will leak memory as the epilogue will not be called as halt() is not going to return.
#[no_mangle]
#[inline(never)]
fn internal_dispatch_function() -> Result<()> {
    reset_error();

    // We should enable this once we have finer tracing control
    // (i.e, we don't go into the guest for every single trace)
    // see https://github.com/deislabs/hyperlight/issues/1215
    // #[cfg(debug_assertions)]
    // crate::trace!("internal_dispatch_function");

    let function_call = try_pop_shared_input_data_into::<FunctionCall>()
        .expect("Function call deserialization failed");

    let result_vec = call_guest_function(&function_call).map_err(|e| {
        set_error(e.kind.clone(), e.message.as_str());
        e
    })?;

    push_shared_output_data(result_vec)
}

// This is implemented as a separate function to make sure that epilogue in the internal_dispatch_function is called before the halt()
// which if it were included in the internal_dispatch_function cause the epilogue to not be called because the halt() would not return
// when running in the hypervisor.
pub(crate) fn dispatch_function() {
    let _ = internal_dispatch_function();
    halt();
}
