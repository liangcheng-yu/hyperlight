#![crate_type = "rlib"]
#![no_std]

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn HyperlightMain() {
    // try to use GetFlatBufferResultFromInt to make sure we can communicate w/ the HyperlightGuest lib
    let _result = unsafe { GetFlatBufferResultFromInt(42) };
}

extern "C" {
    pub fn GetFlatBufferResultFromInt(value: u32) -> *const u8;
}