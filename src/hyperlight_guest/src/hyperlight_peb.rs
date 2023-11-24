#![allow(non_snake_case)]

use core::ffi::{c_char, c_void};

#[repr(C)]
pub struct HostFunctionDefinitions {
    pub fbHostFunctionDetailsSize: u64,
    pub fbHostFunctionDetails: *mut c_void,
}

#[repr(C)]
pub struct HostException {
    pub hostExceptionSize: u64,
}

#[repr(C)]
pub struct InputData {
    pub inputDataSize: u64,
    pub inputDataBuffer: *mut c_void,
}

#[repr(C)]
pub struct OutputData {
    pub outputDataSize: u64,
    pub outputDataBuffer: *mut c_void,
}

#[repr(C)]
pub struct GuestHeapData {
    pub guestHeapSize: u64,
    pub guestHeapBuffer: *mut c_void,
}

#[repr(C)]
pub struct GuestStackData {
    pub minStackAddress: u64,
}

#[repr(C)]
pub struct HyperlightPEB {
    pub security_cookie_seed: u64,
    pub guest_function_dispatch_ptr: u64,
    pub hostFunctionDefinitions: HostFunctionDefinitions,
    pub hostException: HostException,
    pub pGuestErrorBuffer: *mut c_void,
    pub guestErrorBufferSize: u64,
    pub pCode: *mut c_char,
    pub pOutb: *mut c_void,
    pub pOutbContext: *mut c_void,
    pub inputdata: InputData,
    pub outputdata: OutputData,
    pub guestheapData: GuestHeapData,
    pub gueststackData: GuestStackData,
}
