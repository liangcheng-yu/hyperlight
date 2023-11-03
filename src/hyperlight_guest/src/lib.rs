#![no_std]

extern crate alloc;

pub mod hyperlight_peb;
pub mod guest;
#[allow(non_camel_case_types)]
pub mod gen_flatbuffers;

// Wrappers for Flatbuffer types related to guest functions
pub mod function;

#[no_mangle]
pub extern "C" fn __CxxFrameHandler3() {}
#[no_mangle]
pub static _fltused: i32 = 0;