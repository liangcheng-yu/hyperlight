#![no_std]
#![no_main]

use hyperlight_guest_rs::entrypoint::{
    create_function_definition, get_flatbuffer_result_from_int, register_function,
};

#[allow(non_snake_case)]
pub extern "C" fn hyperlight_main() {
    // - manually register smallVar

    // create fxn def
    let smallVarDefinition = create_function_definition("smallVar\0", smallVar as u64, &[]);

    // register fxn def
    register_function(smallVarDefinition);
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn smallVar() -> *const u8 {
    let _buffer: [u8; 2048] = [0; 2048];
    get_flatbuffer_result_from_int(2048)
}