#![no_std]

// Deps
use alloc::vec::Vec;
use buddy_system_allocator::LockedHeap;
use hyperlight_flatbuffers::flatbuffer_wrappers::guest_function_details::GuestFunctionDetails;
use hyperlight_peb::HyperlightPEB;

extern crate alloc;

// Modules
pub mod entrypoint;

pub mod guest_error;
pub mod guest_function_call;
pub mod guest_functions;

pub mod host_error;
pub mod host_function_call;
pub mod host_functions;

pub mod hyperlight_peb;

pub mod flatbuffer_utils;

// Unresolved symbols
#[no_mangle]
pub(crate) extern "C" fn __CxxFrameHandler3() {}
#[no_mangle]
pub(crate) static _fltused: i32 = 0;

// Globals
pub const DEFAULT_GUEST_STACK_SIZE: i32 = 65536; // default stack size

#[global_allocator]
pub(crate) static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

pub(crate) static mut P_PEB: Option<*mut HyperlightPEB> = None;

pub(crate) static mut OS_PAGE_SIZE: u32 = 0;
pub(crate) static mut OUTB_PTR: Option<fn(u16, u8)> = None;
pub(crate) static mut OUTB_PTR_WITH_CONTEXT: Option<fn(*mut core::ffi::c_void, u16, u8)> = None;
pub(crate) static mut RUNNING_IN_HYPERLIGHT: bool = false;

pub(crate) static mut GUEST_FUNCTIONS_BUILDER: GuestFunctionDetails =
    GuestFunctionDetails::new(Vec::new());
pub(crate) static mut GUEST_FUNCTIONS: Vec<u8> = Vec::new();
