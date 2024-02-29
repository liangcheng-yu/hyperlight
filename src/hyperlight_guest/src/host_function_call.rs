use alloc::{format, string::ToString, vec::Vec};
use core::{arch::global_asm, ptr::copy_nonoverlapping, slice::from_raw_parts};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::{FunctionCall, FunctionCallType},
    function_types::{ParameterValue, ReturnType, ReturnValue},
    guest_error::ErrorCode,
};

use crate::{
    error::{HyperlightGuestError, Result},
    flatbuffer_utils::get_flatbuffer_result_from_int,
    host_error::check_for_host_error,
    host_functions::validate_host_function_call,
    OUTB_PTR, OUTB_PTR_WITH_CONTEXT, P_PEB, RUNNING_IN_HYPERLIGHT,
};

pub enum OutBAction {
    Log = 99,
    CallFunction = 101,
    Abort = 102,
}

pub fn get_host_value_return_as_int() -> Result<i32> {
    let peb_ptr = unsafe { P_PEB.unwrap() };

    let idb = unsafe {
        from_raw_parts(
            (*peb_ptr).inputdata.inputDataBuffer as *mut u8,
            (*peb_ptr).inputdata.inputDataSize as usize,
        )
    };

    // if buffer size is zero, error out
    if idb.is_empty() {
        return Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            "Got a 0-size buffer in GetHostReturnValueAsInt".to_string(),
        ));
    }

    let fcr = if let Ok(r) = ReturnValue::try_from(idb) {
        r
    } else {
        return Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            "Could not convert buffer to ReturnValue in GetHostReturnValueAsInt".to_string(),
        ));
    };

    // check that return value is an int and return
    if let ReturnValue::Int(i) = fcr {
        Ok(i)
    } else {
        Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            "Host return value was not an int as expected".to_string(),
        ))
    }
}

// TODO: Make this generic, return a Result<T, ErrorCode>

pub fn get_host_value_return_as_vecbytes() -> Result<Vec<u8>> {
    let peb_ptr = unsafe { P_PEB.unwrap() };

    let idb = unsafe {
        from_raw_parts(
            (*peb_ptr).inputdata.inputDataBuffer as *mut u8,
            (*peb_ptr).inputdata.inputDataSize as usize,
        )
    };

    // if buffer size is zero, error out
    if idb.is_empty() {
        return Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            "Got a 0-size buffer in GetHostReturnValueAsVecBytes".to_string(),
        ));
    }

    let fcr = if let Ok(r) = ReturnValue::try_from(idb) {
        r
    } else {
        return Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            "Could not convert buffer to ReturnValue in GetHostReturnValueAsVecBytes".to_string(),
        ));
    };

    // check that return value is an Vec<u8> and return
    if let ReturnValue::VecBytes(v) = fcr {
        Ok(v)
    } else {
        Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            "Host return value was not an VecBytes as expected".to_string(),
        ))
    }
}

// TODO: Make this generic, return a Result<T, ErrorCode> this should allow callers to call this function and get the result type they expect
// without having to do the conversion themselves

pub fn call_host_function(
    function_name: &str,
    parameters: Option<Vec<ParameterValue>>,
    return_type: ReturnType,
) -> Result<()> {
    unsafe {
        let peb_ptr = P_PEB.unwrap();

        let host_function_call = FunctionCall::new(
            function_name.to_string(),
            parameters,
            FunctionCallType::Host,
            return_type,
        );

        validate_host_function_call(&host_function_call)?;

        let host_function_call_buffer: Vec<u8> = host_function_call.try_into().unwrap();
        let host_function_call_buffer_size = host_function_call_buffer.len();

        if host_function_call_buffer_size as u64 > (*peb_ptr).outputdata.outputDataSize {
            return Err(HyperlightGuestError::new(
                ErrorCode::GuestError,
                format!(
                    "Host Function Call Buffer is too big ({}) for output data ({}) Function Name: {}",
                    host_function_call_buffer_size, (*peb_ptr).outputdata.outputDataSize, function_name
                ),
            ));
        }

        let output_data_buffer = (*peb_ptr).outputdata.outputDataBuffer as *mut u8;

        copy_nonoverlapping(
            host_function_call_buffer.as_ptr(),
            output_data_buffer,
            host_function_call_buffer_size,
        );

        outb(OutBAction::CallFunction as u16, 0);

        Ok(())
    }
}

pub fn outb(port: u16, value: u8) {
    unsafe {
        if RUNNING_IN_HYPERLIGHT {
            hloutb(port, value);
        } else if let Some(outb_func) = OUTB_PTR_WITH_CONTEXT {
            if let Some(peb_ptr) = P_PEB {
                outb_func((*peb_ptr).pOutbContext, port, value);
            }
        } else if let Some(outb_func) = OUTB_PTR {
            outb_func(port, value);
        }

        check_for_host_error();
    }
}

extern "win64" {
    fn hloutb(port: u16, value: u8);
}

pub fn print_output_as_guest_function(function_call: &FunctionCall) -> Result<Vec<u8>> {
    if let ParameterValue::String(message) = function_call.parameters.clone().unwrap()[0].clone() {
        call_host_function(
            "HostPrint",
            Some(Vec::from(&[ParameterValue::String(message.to_string())])),
            ReturnType::Int,
        )?;
        let res_i = get_host_value_return_as_int()?;
        Ok(get_flatbuffer_result_from_int(res_i))
    } else {
        Err(HyperlightGuestError::new(
            ErrorCode::GuestError,
            "Wrong Parameters passed to print_output_as_guest_function".to_string(),
        ))
    }
}

// port: RCX(cx), value: RDX(dl)
global_asm!(
    ".global hloutb
        hloutb:
            xor rax, rax
            mov al, dl
            mov dx, cx
            out dx, al
            ret"
);
