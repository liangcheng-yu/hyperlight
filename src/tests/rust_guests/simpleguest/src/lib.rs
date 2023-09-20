#![crate_type = "rlib"]
#![no_std]

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn HyperlightMain() {
    // - manually register smallVar

    // get name
    let smallVarName: [u8; 9] = *b"smallVar\0";
    let smallVarNamePtr: *const u8 = &smallVarName as *const u8;

    // get params
    let smallParameterKind: [u8; 1] = [0];
    let smallParameterKindPtr: *const u8 = &smallParameterKind as *const u8;

    // create fxn def
    let smallVarDefinition = unsafe { CreateFunctionDefinition(smallVarNamePtr, smallVar, 0, smallParameterKindPtr) };

    // register fxn def
    unsafe { RegisterFunction(smallVarDefinition) };
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn smallVar() -> *const u8 {
    let _buffer: [u8; 2048] = [0; 2048];
    return unsafe { GetFlatBufferResultFromInt(2048) };
}

extern "C" {
    #[allow(non_snake_case)]
    pub fn GetFlatBufferResultFromInt(value: u32) -> *const u8;

    #[allow(non_snake_case)]
    pub fn CreateFunctionDefinition(functionName: *const u8, pFunction: extern "C" fn() -> *const u8, paramCount: i32, parameterKind: *const u8) -> i32;
    // ^^^ In C, this function returns ns(GuestFunctionDefinition_ref_t),
    // which is utilizing a macro from flatbuffers to expand to 
    // Hyperlight_Generated_GuestFunctionDefinition_ref_t,
    // which, in itself, is flatbuffers_ref_t,
    // which is just a typedef for flatcc_builder_ref_t
    // which is a wrapper for flatcc_builder_ref_t
    // that expands to flatbuffers_soffset_t
    // which, finally, is a typedef for int32_t.

    #[allow(non_snake_case)]
    pub fn RegisterFunction(functionDefinition: i32);
    // ^^^ In C, this function takes ns(GuestFunctionDefinition_ref_t),
    // but we can follow the same logic as the function above to use i32 instead.
}