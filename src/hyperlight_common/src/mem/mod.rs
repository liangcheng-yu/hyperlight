#![allow(non_snake_case)]

pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: u64 = 1 << 12;
pub const PAGE_SIZE_USIZE: usize = 1 << 12;

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
pub struct GuestErrorData {
    pub guestErrorSize: u64,
    pub guestErrorBuffer: *mut c_void,
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
pub struct GuestPanicContextData {
    pub guestPanicContextDataSize: u64,
    pub guestPanicContextDataBuffer: *mut c_void,
}

#[repr(C)]
pub struct HyperlightPEB {
    pub security_cookie_seed: u64,
    pub guest_function_dispatch_ptr: u64,
    pub hostFunctionDefinitions: HostFunctionDefinitions,
    pub hostException: HostException,
    pub guestErrorData: GuestErrorData,
    pub pCode: *mut c_char,
    pub pOutb: *mut c_void,
    pub pOutbContext: *mut c_void,
    pub inputdata: InputData,
    pub outputdata: OutputData,
    pub guestPanicContextData: GuestPanicContextData,
    pub guestheapData: GuestHeapData,
    pub gueststackData: GuestStackData,
}
