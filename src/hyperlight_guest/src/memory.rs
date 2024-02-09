use core::alloc::Layout;
use core::ffi::c_void;
use core::mem::size_of;
use hyperlight_flatbuffers::flatbuffer_wrappers::guest_error::ErrorCode;

use crate::entrypoint::abort_with_code;

extern crate alloc;

const ALIGN: usize = 8;

#[no_mangle]
pub extern "C" fn hlmalloc(size: usize) -> *const c_void {
    // Allocate a block that includes space for both layout information and data
    let total_size = size + size_of::<Layout>();
    let layout = Layout::from_size_align(total_size, ALIGN).unwrap();
    unsafe {
        let raw_ptr = alloc::alloc::alloc(layout);
        if raw_ptr.is_null() {
            abort_with_code(ErrorCode::MallocFailed as i32);
            core::ptr::null()
        } else {
            let layout_ptr = raw_ptr as *mut Layout;
            layout_ptr.write(layout);
            layout_ptr.add(1) as *const c_void
        }
    }
}

#[no_mangle]
pub extern "C" fn hlcalloc(n: usize, size: usize) -> *const c_void {
    let total_size = n * size;
    hlmalloc(total_size)
}

#[no_mangle]
pub extern "C" fn hlfree(ptr: *const c_void) {
    if !ptr.is_null() {
        unsafe {
            let block_start = (ptr as *const Layout).sub(1);
            let layout = block_start.read();
            alloc::alloc::dealloc(block_start as *mut u8, layout);
        }
    }
}

#[no_mangle]
pub extern "C" fn hlrealloc(ptr: *const c_void, size: usize) -> *const c_void {
    if ptr.is_null() {
        // If the pointer is null, treat as a malloc
        return hlmalloc(size);
    }

    unsafe {
        let block_start = (ptr as *const Layout).sub(1);
        let layout = block_start.read();
        let total_new_size = size + size_of::<Layout>();
        let new_block_start =
            alloc::alloc::realloc(block_start as *mut u8, layout, total_new_size) as *mut Layout;

        if new_block_start.is_null() {
            // Realloc failed
            abort_with_code(ErrorCode::MallocFailed as i32);
            core::ptr::null()
        } else {
            // Return the pointer just after the layout information
            // since old layout should still as it would have been copied
            new_block_start.add(1) as *const c_void
        }
    }
}
