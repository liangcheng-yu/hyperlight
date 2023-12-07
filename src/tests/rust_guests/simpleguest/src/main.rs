#![no_std]
#![no_main]

extern crate alloc;

use alloc::{string::ToString, vec::Vec};
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::FunctionCall,
    function_types::{ParameterType, ParameterValue, ReturnType},
    guest_function_definition::GuestFunctionDefinition,
};
use hyperlight_guest::{
    entrypoint::halt,
    flatbuffer_utils::get_flatbuffer_result_from_int,
    guest_functions::register_function,
    host_function_call::{call_host_function, get_host_value_return_as_int},
};

extern crate hyperlight_guest;

#[no_mangle]
#[allow(improper_ctypes_definitions, non_camel_case_types)]
pub extern "C" fn simple_print_output(function_call: &FunctionCall) -> Vec<u8> {
    if let ParameterValue::String(message) = function_call.parameters.clone().unwrap()[0].clone() {
        call_host_function(
            "HostPrint",
            Some(Vec::from(&[ParameterValue::String(message)])),
            ReturnType::Int,
        );
        let result = get_host_value_return_as_int();
        get_flatbuffer_result_from_int(result)
    } else {
        Vec::new()
    }
}

// set_byte_array_to_zero

// print_two_args

// print_three_args

// print_four_args

// print_five_args

// log_to_host

// print_six_args

// print_seven_args

// print_eight_args

// print_nine_args

// print_ten_args

// stack_allocate

// buffer_overrun

// stack_overflow

// large_var

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn small_var(_: &FunctionCall) -> Vec<u8> {
    let _buffer: [u8; 1024] = [0; 1024];
    get_flatbuffer_result_from_int(1024)
}

// call_malloc

// malloc_and_free

// echo

// get_size_prefixed_buffer

// spin

#[no_mangle]
pub extern "C" fn hyperlight_main() {
    let small_var_def = GuestFunctionDefinition::new(
        "SmallVar".to_string(),
        Vec::new(),
        ReturnType::Int,
        small_var as i64,
    );
    register_function(small_var_def);

    let simple_print_output_def = GuestFunctionDefinition::new(
        "PrintOutput".to_string(),
        Vec::from(&[ParameterType::String]),
        ReturnType::Int,
        simple_print_output as i64,
    );
    register_function(simple_print_output_def);
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn guest_dispatch_function() -> Vec<u8> {
    [0; 0].to_vec()
}

// It looks like rust-analyzer doesn't correctly manage no_std crates,
// and so it displays an error about a duplicate panic_handler.
// See more here: https://github.com/rust-lang/rust-analyzer/issues/4490
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    halt();
    loop {}
}
