#![no_std]
// Deps
use alloc::vec::Vec;
use buddy_system_allocator::LockedHeap;
use core::arch::global_asm;
use entrypoint::abort;
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
pub mod memory;
pub mod print;
pub(crate) mod security_check;
pub mod setjmp;

pub mod logging;

// Unresolved symbols
#[no_mangle]
pub(crate) extern "C" fn __CxxFrameHandler3() {}
#[no_mangle]
pub(crate) static _fltused: i32 = 0;

// __security_cookie
#[no_mangle]
pub(crate) static mut __security_cookie: u64 = 0;

// It looks like rust-analyzer doesn't correctly manage no_std crates,
// and so it displays an error about a duplicate panic_handler.
// See more here: https://github.com/rust-lang/rust-analyzer/issues/4490
// The cfg_attr attribute is used to avoid clippy failures as test pulls in std which pulls in a panic handler
#[cfg_attr(not(test), panic_handler)]
#[allow(clippy::panic)]
// to satisfy the clippy when cfg == test
#[allow(dead_code)]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // TODO: Rather than abort, we should probably abort with a code and potentially write some context to shared memory.
    abort()
}

extern "win64" {
    fn __chkstk();
    fn __chkstk_in_proc();
}

global_asm!(
    "
    .global __chkstk
    __chkstk:
        /* Make space on the stack and save R10 and R11 */
        sub rsp, 0x10
        mov qword ptr [rsp], r10
        mov qword ptr [rsp + 8], r11
        /* Check if we are running in Hyperlight */
        lea r11,[rip+{running_in_hyperlight}]
        movzx r11,byte ptr [r11]
        test r11,r11
        /* If we are not running in Hyperlight, jump to call_chk_inproc */
        je call_chk_inproc
        /* Load the minimum stack address from the PEB */
        lea r11,[rip+{min_stack_addr}]  
        mov r11, qword ptr [r11]
        /* Get the current stack pointer */
        lea r10,[rsp+0x18]  
        /* Calculate what the new stack pointer will be */
        sub r10, rax
        /* Compare the new stack pointer with the minimum stack address */
        cmp r10,r11   
        /* If the new stack pointer is above the minimum stack address, jump to cs_ret */
        jae cs_ret
        /* If the new stack pointer is below the minimum stack address, 
        then set the error code to 9 (stack overflow) call set_error and halt */
        call {set_error}
        hlt
    call_chk_inproc:
        call {chkstk_in_proc}
    cs_ret:
        /* Restore R10 and R11 and return */
        mov r10, qword ptr [rsp]
        mov r11, qword ptr [rsp + 8]
        add rsp, 0x10
        ret
",
        running_in_hyperlight = sym RUNNING_IN_HYPERLIGHT,
        chkstk_in_proc = sym __chkstk_in_proc,
        min_stack_addr = sym MIN_STACK_ADDRESS,
        set_error = sym set_stack_allocate_error,

);

global_asm!(
    "
    .global __chkstk_in_proc
    __chkstk_in_proc:
        /* Get the current stack pointer */
        lea r10, [rsp + 0x18]
        /* Calculate what the new stack pointer will be */
        sub r10, rax
        cmovb r10, r11
        mov r11, qword ptr gs:[0x0000000000000010]
        cmp r10, r11
        jae csip_ret
        and r10w,0x0F000
    csip_stackprobe:
        lea r11, [r11 + 0x0FFFFFFFFFFFFF000]
        mov byte ptr [r11], 0
        cmp r10, r11
        jne csip_stackprobe
    csip_ret:
        ret
",
);

// Globals

#[derive(Debug)]
pub struct HyperlightGuestError;

#[global_allocator]
pub(crate) static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

pub(crate) static mut P_PEB: Option<*mut HyperlightPEB> = None;
pub(crate) static mut MIN_STACK_ADDRESS: u64 = 0;

pub(crate) static mut OS_PAGE_SIZE: u32 = 0;
pub(crate) static mut OUTB_PTR: Option<fn(u16, u8)> = None;
pub(crate) static mut OUTB_PTR_WITH_CONTEXT: Option<fn(*mut core::ffi::c_void, u16, u8)> = None;
pub(crate) static mut RUNNING_IN_HYPERLIGHT: bool = false;

pub(crate) static mut GUEST_FUNCTIONS_BUILDER: GuestFunctionDetails =
    GuestFunctionDetails::new(Vec::new());
pub(crate) static mut GUEST_FUNCTIONS: Vec<u8> = Vec::new();

#[no_mangle]
extern "win64" fn set_stack_allocate_error() {
    guest_error::set_error(
        hyperlight_flatbuffers::flatbuffer_wrappers::guest_error::ErrorCode::StackOverflow,
        "Stack Overflow",
    )
}
