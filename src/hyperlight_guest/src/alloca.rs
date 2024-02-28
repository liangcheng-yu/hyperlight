extern "C" {
    fn _alloca_wrapper(size: usize) -> *mut u8;
}

pub fn _alloca(size: usize) -> *mut u8 {
    unsafe { _alloca_wrapper(size) }
}
