#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

// It looks like rust-analyzer doesn't correctly manage no_std crates,
// and so it displays an error about a duplicate panic_handler.
// See more here: https://github.com/rust-lang/rust-analyzer/issues/4490
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt();
    loop {}
}

fn halt() {
    unsafe {
        asm!("hlt");
    }
}

fn mmio_read() {
    unsafe {
        asm!("mov al, [0x8000]");
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn entrypoint(a: i64, b: i64, c: i32) -> i32 {
    if a != 0x230000 || b != 1234567890 || c != 4096 {
        mmio_read();
    }
    halt();
    0
}
