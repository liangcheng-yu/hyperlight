#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use hyperlight_guest::guest::{
    create_function_definition, get_flatbuffer_result_from_int, register_function,
};

extern crate hyperlight_guest;

#[no_mangle]
pub extern "C" fn hyperlight_main() {
    // create fxn def
    let small_var_def = create_function_definition("small_var", small_var as i64, &[]);

    // register fxn def
    register_function(small_var_def);
}

#[no_mangle]
pub extern "C" fn small_var() -> Vec<u8> {
    let _buffer: [u8; 2048] = [0; 2048];
    get_flatbuffer_result_from_int(2048)
}

#[no_mangle]
pub extern "C" fn guest_dispatch_function() -> Vec<u8> {
    [0; 0].to_vec()
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
