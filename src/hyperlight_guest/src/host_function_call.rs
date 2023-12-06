use core::{arch::asm, ptr::copy_nonoverlapping, slice::from_raw_parts};

use alloc::{format, string::ToString, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::{FunctionCall, FunctionCallType},
    function_types::{ParameterValue, ReturnType, ReturnValue},
    guest_error::ErrorCode,
};

use crate::{
    guest_error::set_error, host_error::check_for_host_error,
    host_functions::validate_host_function_call, OUTB_PTR, OUTB_PTR_WITH_CONTEXT, P_PEB,
    RUNNING_IN_HYPERLIGHT,
};

pub enum OutBAction {
    Log = 99,
    CallFunction = 101,
    Abort = 102,
}

pub fn get_host_value_return_as_int() -> i32 {
    let peb_ptr = unsafe { P_PEB.unwrap() };

    let idb = unsafe {
        from_raw_parts(
            (*peb_ptr).inputdata.inputDataBuffer as *mut u8,
            (*peb_ptr).inputdata.inputDataSize as usize,
        )
    };

    // if buffer size is zero, error out
    if idb.is_empty() {
        set_error(
            ErrorCode::GuestError,
            "Got a 0-size buffer in GetHostReturnValueAsInt",
        );
        return -1;
    }

    let fcr = if let Ok(r) = ReturnValue::try_from(idb) {
        r
    } else {
        set_error(
            ErrorCode::GuestError,
            "Could not convert buffer to ReturnValue in GetHostReturnValueAsInt",
        );
        return -1;
    };

    // check that return value is an int and return
    if let ReturnValue::Int(i) = fcr {
        i
    } else {
        set_error(
            ErrorCode::GuestError,
            "Host return value was not an int as expected",
        );
        -1
    }
}

pub fn call_host_function(
    function_name: &str,
    parameters: Option<Vec<ParameterValue>>,
    return_type: ReturnType,
) {
    unsafe {
        let peb_ptr = P_PEB.unwrap();

        let host_function_call = FunctionCall::new(
            function_name.to_string(),
            parameters,
            FunctionCallType::Host,
            return_type,
        );

        // validate host functions
        validate_host_function_call(&host_function_call);

        let host_function_call_buffer: Vec<u8> = host_function_call.try_into().unwrap();
        let host_function_call_buffer_size = host_function_call_buffer.len();

        if host_function_call_buffer_size as u64 > (*peb_ptr).outputdata.outputDataSize {
            set_error(
                ErrorCode::GuestError,
                &format!(
                "Host Function Call Buffer is too big ({}) for output data ({}) Function Name: {}",
                host_function_call_buffer_size, (*peb_ptr).outputdata.outputDataSize, function_name
            ),
            );
            return;
        }

        let output_data_buffer = (*peb_ptr).outputdata.outputDataBuffer as *mut u8;

        copy_nonoverlapping(
            host_function_call_buffer.as_ptr(),
            output_data_buffer,
            host_function_call_buffer_size,
        );

        outb(OutBAction::CallFunction as u16, 0);
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

pub fn hloutb(port: u16, value: u8) {
    unsafe {
        asm!(
            "mov al, {value}",
            "mov dx, {port:x}",
            "out dx, al",
            port = in(reg) port,
            value = in(reg_byte) value,
        );
    }
}
