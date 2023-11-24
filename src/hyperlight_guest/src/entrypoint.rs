use crate::{
    guest_error::reset_error, guest_function_call::dispatch_function,
    guest_functions::finalise_function_table, hyperlight_peb::HyperlightPEB, HEAP_ALLOCATOR,
    OS_PAGE_SIZE, OUTB_PTR, OUTB_PTR_WITH_CONTEXT, P_PEB, RUNNING_IN_HYPERLIGHT,
};

use core::{arch::asm, ffi::c_void};

pub fn halt() {
    unsafe {
        asm!("hlt");
    }
}

extern "C" {
    fn hyperlight_main();
}

#[no_mangle]
pub extern "C" fn entrypoint(peb_address: u64, _seed: u64, ops: i32) -> i32 {
    unsafe {
        if peb_address == 0 {
            return -1;
        }

        P_PEB = Some(peb_address as *mut HyperlightPEB);

        let peb_ptr = P_PEB.unwrap();

        let heap_start = (*peb_ptr).guestheapData.guestHeapBuffer as usize;
        let heap_size = (*peb_ptr).guestheapData.guestHeapSize as usize;
        HEAP_ALLOCATOR.lock().init(heap_start, heap_size);

        // In C, at this point, we call __security_init_cookie.
        // That's a dependency on MSVC, which we can't utilize here.
        // This is to protect against buffer overflows in C, which
        // are inherently protected in Rust.

        // In C, here, we have a `if (!setjmp(jmpbuf))`, which is used in case an error occurs
        // because longjmp is called, which will cause execution to return to this point to
        // halt the program. In Rust, we don't have or need this sort of error handling as the
        // language relies on specific structures like `Result`, and `?` that allow for
        // propagating up the call stack.

        OS_PAGE_SIZE = ops as u32;

        let outb_ptr: fn(u16, u8) = core::mem::transmute((*peb_ptr).pOutb);
        OUTB_PTR = Some(outb_ptr as fn(u16, u8));

        let outb_ptr_with_context: fn(*mut c_void, u16, u8) =
            core::mem::transmute((*peb_ptr).pOutb);
        OUTB_PTR_WITH_CONTEXT = Some(outb_ptr_with_context as fn(*mut c_void, u16, u8));

        if !(*peb_ptr).pOutb.is_null() {
            RUNNING_IN_HYPERLIGHT = true;
        }

        (*peb_ptr).guest_function_dispatch_ptr = dispatch_function as u64;

        reset_error();

        hyperlight_main();

        finalise_function_table();
    }

    // halt?
    0
}
