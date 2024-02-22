#![no_std]
#![no_main]

extern crate alloc;
extern crate hyperlight_guest;

use alloc::{format, string::ToString, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::FunctionCall,
    function_types::{ParameterType, ParameterValue, ReturnType},
    guest_error::ErrorCode,
    guest_function_definition::GuestFunctionDefinition,
    guest_log_level::LogLevel,
};
use hyperlight_guest::{
    flatbuffer_utils::{get_flatbuffer_result_from_int, get_flatbuffer_result_from_void},
    guest_error::set_error,
    guest_functions::register_function,
    host_function_call::{
        call_host_function, get_host_value_return_as_int, print_output_as_guest_function,
    },
    logging::log,
};

fn send_message_to_host_method(
    method_name: &str,
    guest_message: &str,
    message: &str,
) -> Result<Vec<u8>, ()> {
    let message = format!("{}{}", guest_message, message);
    if let Err(_) = call_host_function(
        method_name,
        Some(Vec::from(&[ParameterValue::String(message.to_string())])),
        ReturnType::Int,
    ) {
        return Err(());
    }
    let result = get_host_value_return_as_int();

    Ok(get_flatbuffer_result_from_int(result))
}

fn guest_function(function_call: &FunctionCall) -> Vec<u8> {
    if let ParameterValue::String(message) = &function_call.parameters.as_ref().unwrap()[0] {
        match send_message_to_host_method("HostMethod", "Hello from GuestFunction, ", message) {
            Ok(result) => result,
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

fn guest_function1(function_call: &FunctionCall) -> Vec<u8> {
    if let ParameterValue::String(message) = &function_call.parameters.as_ref().unwrap()[0] {
        match send_message_to_host_method("HostMethod1", "Hello from GuestFunction1, ", message) {
            Ok(result) => result,
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

fn guest_function2(function_call: &FunctionCall) -> Vec<u8> {
    if let ParameterValue::String(message) = &function_call.parameters.as_ref().unwrap()[0] {
        match send_message_to_host_method("HostMethod1", "Hello from GuestFunction2, ", message) {
            Ok(result) => result,
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

fn guest_function3(function_call: &FunctionCall) -> Vec<u8> {
    if let ParameterValue::String(message) = &function_call.parameters.as_ref().unwrap()[0] {
        match send_message_to_host_method("HostMethod1", "Hello from GuestFunction3, ", message) {
            Ok(result) => result,
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

fn guest_function4() -> Vec<u8> {
    call_host_function(
        "HostMethod4",
        Some(Vec::from(&[ParameterValue::String(
            "Hello from GuestFunction4".to_string(),
        )])),
        ReturnType::Void,
    ).unwrap();

    get_flatbuffer_result_from_void()
}

fn log_message(function_call: &FunctionCall) -> Vec<u8> {
    if let (
        ParameterValue::String(message),
        ParameterValue::String(source),
        ParameterValue::Int(level),
    ) = (
        &function_call.parameters.as_ref().unwrap()[0],
        &function_call.parameters.as_ref().unwrap()[1],
        &function_call.parameters.as_ref().unwrap()[2],
    ) {
        let mut log_level = *level;
        if log_level < 0 || log_level > 6 {
            log_level = 0;
        }

        log(
            LogLevel::from(log_level as u8),
            message,
            source,
            "log_message",
            file!(),
            line!(),
        );

        get_flatbuffer_result_from_int(message.len() as i32)
    } else {
        Vec::new()
    }
}

fn call_error_method(function_call: &FunctionCall) -> Vec<u8> {
    if let ParameterValue::String(message) = &function_call.parameters.as_ref().unwrap()[0] {
        match send_message_to_host_method("ErrorMethod", "Error From Host: ", message) {
            Ok(result) => result,
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

fn call_host_spin() -> Vec<u8> {
    call_host_function("Spin", None, ReturnType::Void).unwrap();
    get_flatbuffer_result_from_void()
}

#[no_mangle]
pub extern "C" fn hyperlight_main() {
    let print_output_def = GuestFunctionDefinition::new(
        "PrintOutput".to_string(),
        Vec::from(&[ParameterType::String]),
        ReturnType::Int,
        print_output_as_guest_function as i64,
    );
    register_function(print_output_def);

    let guest_function_def = GuestFunctionDefinition::new(
        "GuestMethod".to_string(),
        Vec::from(&[ParameterType::String]),
        ReturnType::Int,
        guest_function as i64,
    );
    register_function(guest_function_def);

    let guest_function1_def = GuestFunctionDefinition::new(
        "GuestMethod1".to_string(),
        Vec::from(&[ParameterType::String]),
        ReturnType::Int,
        guest_function1 as i64,
    );
    register_function(guest_function1_def);

    let guest_function2_def = GuestFunctionDefinition::new(
        "GuestMethod2".to_string(),
        Vec::from(&[ParameterType::String]),
        ReturnType::Int,
        guest_function2 as i64,
    );
    register_function(guest_function2_def);

    let guest_function3_def = GuestFunctionDefinition::new(
        "GuestMethod3".to_string(),
        Vec::from(&[ParameterType::String]),
        ReturnType::Int,
        guest_function3 as i64,
    );
    register_function(guest_function3_def);

    let guest_function4_def = GuestFunctionDefinition::new(
        "GuestMethod4".to_string(),
        Vec::new(),
        ReturnType::Int,
        guest_function4 as i64,
    );
    register_function(guest_function4_def);

    let log_message_def = GuestFunctionDefinition::new(
        "LogMessage".to_string(),
        Vec::from(&[
            ParameterType::String,
            ParameterType::String,
            ParameterType::Int,
        ]),
        ReturnType::Int,
        log_message as i64,
    );
    register_function(log_message_def);

    let call_error_method_def = GuestFunctionDefinition::new(
        "CallErrorMethod".to_string(),
        Vec::from(&[ParameterType::String]),
        ReturnType::Int,
        call_error_method as i64,
    );
    register_function(call_error_method_def);

    let call_host_spin_def = GuestFunctionDefinition::new(
        "CallHostSpin".to_string(),
        Vec::new(),
        ReturnType::Int,
        call_host_spin as i64,
    );
    register_function(call_host_spin_def);
}

#[no_mangle]
pub fn guest_dispatch_function(function_call: &FunctionCall) -> Vec<u8> {
    set_error(
        ErrorCode::GuestFunctionNotFound,
        &function_call.function_name,
    );
    Vec::new()
}
