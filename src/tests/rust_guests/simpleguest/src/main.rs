#![no_std]
#![no_main]

use hyperlight_guest::{guest::{
    create_function_definition, get_flatbuffer_result_from_int, register_function,
}, flatbuffers::hyperlight::generated::FunctionCall};

extern crate hyperlight_guest;

#[no_mangle]
pub extern "C" fn hyperlight_main() {
    // create fxn def
    let small_var_def = create_function_definition("small_var\0", small_var as u64, &[]);

    // register fxn def
    register_function(small_var_def);
}

#[no_mangle]
pub extern "C" fn small_var() -> *const u8 {
    let _buffer: [u8; 2048] = [0; 2048];
    get_flatbuffer_result_from_int(2048)
}

#[no_mangle]
pub extern "C" fn guest_dispatch_function(_function_call: &FunctionCall) -> *mut u8 {
    // return dummy value for now
    0 as *mut u8
}
