#![no_std]

extern crate alloc;

pub mod hyperlight_peb;
pub mod guest;

#[no_mangle]
pub extern "C" fn __CxxFrameHandler3() {}
#[no_mangle]
pub static _fltused: i32 = 0;