use core::{ptr::copy_nonoverlapping, arch::asm};

use alloc::{format, string::ToString, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::{FunctionCall, FunctionCallType},
    function_types::{ParameterValue, ReturnType},
    guest_error::ErrorCode,
};

use crate::{guest_error::set_error, host_functions::validate_host_function_call, P_PEB, RUNNING_IN_HYPERLIGHT, OUTB_PTR_WITH_CONTEXT, OUTB_PTR, host_error::check_for_host_error};

pub enum OutBAction {
    Log = 99,
    CallFunction = 101,
    Abort = 102,
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

pub unsafe fn outb(port: u16, value: u8) {
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
