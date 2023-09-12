#![no_std]
#![no_main]

use core::panic::PanicInfo;

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt();
    loop {}
}

fn halt() {
    let hlt: [u8; 1] = [0xF4];
    let func: unsafe extern "C" fn() = unsafe { core::mem::transmute(&hlt as *const _) };
    unsafe {
        func();
    }
}

// Equivalent of the mmio_read function
fn mmio_read() {
    let mmio_read: [u8; 4] = [0x8a, 0x16, 0x00, 0x80];
    let func: unsafe extern "C" fn() = unsafe { core::mem::transmute(&mmio_read as *const _) };
    unsafe {
        func();
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn entryPoint(a: i64, b: i64, c: i32) -> i32 {
    // Check that expected values were passed in
    if a != 0x230000 || b != 1234567890 || c != 4096 {
        mmio_read();
    }
    halt();
    0
}
