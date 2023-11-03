use core::ffi::{c_void, c_char};

use alloc::vec::Vec;

#[repr(C)]
pub struct HostFunctionDefinitions {
    pub fb_host_function_details_size: u64,
    pub fb_host_function_details: *mut c_void,
}

#[repr(C)]
pub struct HostException {
    pub host_exception_size: u64,
}

#[repr(C)]
pub struct InputData {
    pub input_data_size: u64,
    pub input_data_buffer: Vec<u8>,
}

#[repr(C)]
pub struct OutputData {
    pub output_data_size: u64,
    pub output_data_buffer: *mut c_void,
}

#[repr(C)]
pub struct GuestHeapData {
    pub guest_heap_size: u64,
    pub guest_heap_buffer: *mut c_void,
}

#[repr(C)]
pub struct GuestStackData {
    pub min_stack_address: u64,
}

#[repr(C)]
pub struct HyperlightPEB {
    pub security_cookie_seed: u64,
    pub guest_function_dispatch_ptr: u64,
    pub host_function_definitions: HostFunctionDefinitions,
    pub host_exception: HostException,
    pub p_guest_error_buffer: *mut c_void,
    pub guest_error_buffer_size: u64,
    pub p_code: *mut c_char,
    pub p_outb: *mut c_void,
    pub p_outb_context: *mut c_void,
    pub input_data: InputData,
    pub output_data: OutputData,
    pub guest_heap_data: GuestHeapData,
    pub guest_stack_data: GuestStackData,
}
