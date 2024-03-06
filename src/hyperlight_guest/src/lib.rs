#![no_std]
// Deps
use crate::host_function_call::{outb, OutBAction};
use alloc::{string::ToString, vec::Vec};
use buddy_system_allocator::LockedHeap;
use core::hint::unreachable_unchecked;
use core::ptr::copy_nonoverlapping;
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

pub mod alloca;
pub mod flatbuffer_utils;
pub mod memory;
pub mod print;
pub(crate) mod security_check;
pub mod setjmp;

pub mod chkstk;
pub mod error;
pub mod logging;

// Unresolved symbols
#[no_mangle]
pub(crate) extern "C" fn __CxxFrameHandler3() {}
#[no_mangle]
pub(crate) static _fltused: i32 = 0;

// It looks like rust-analyzer doesn't correctly manage no_std crates,
// and so it displays an error about a duplicate panic_handler.
// See more here: https://github.com/rust-lang/rust-analyzer/issues/4490
// The cfg_attr attribute is used to avoid clippy failures as test pulls in std which pulls in a panic handler
#[cfg_attr(not(test), panic_handler)]
#[allow(clippy::panic)]
// to satisfy the clippy when cfg == test
#[allow(dead_code)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        let peb_ptr = P_PEB.unwrap();
        copy_nonoverlapping(
            info.to_string().as_ptr(),
            (*peb_ptr).guestPanicContextData.guestPanicContextDataBuffer as *mut u8,
            (*peb_ptr).guestPanicContextData.guestPanicContextDataSize as usize,
        );
    }
    outb(OutBAction::Abort as u16, 0x0 as u8);
    unsafe { unreachable_unchecked() }
}

// Globals
#[global_allocator]
pub(crate) static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

#[no_mangle]
pub(crate) static mut __security_cookie: u64 = 0;

pub(crate) static mut P_PEB: Option<*mut HyperlightPEB> = None;
pub(crate) static mut MIN_STACK_ADDRESS: u64 = 0;

pub(crate) static mut OS_PAGE_SIZE: u32 = 0;
pub(crate) static mut OUTB_PTR: Option<fn(u16, u8)> = None;
pub(crate) static mut OUTB_PTR_WITH_CONTEXT: Option<fn(*mut core::ffi::c_void, u16, u8)> = None;
pub(crate) static mut RUNNING_IN_HYPERLIGHT: bool = false;

pub(crate) static mut GUEST_FUNCTIONS_BUILDER: GuestFunctionDetails =
    GuestFunctionDetails::new(Vec::new());
pub(crate) static mut GUEST_FUNCTIONS: Vec<u8> = Vec::new();
