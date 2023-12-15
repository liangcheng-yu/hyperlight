use core::slice::from_raw_parts;

use alloc::vec::Vec;
use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use hyperlight_flatbuffers::flatbuffers::hyperlight::generated::{
    hlint as Fbhlint, hlintArgs as FbhlintArgs, hlsizeprefixedbuffer as Fbhlsizeprefixedbuffer,
    hlsizeprefixedbufferArgs as FbhlsizeprefixedbufferArgs, hlstring as Fbhlstring,
    hlstringArgs as FbhlstringArgs, hlvoid as Fbhlvoid, hlvoidArgs as FbhlvoidArgs,
    FunctionCallResult as FbFunctionCallResult, FunctionCallResultArgs as FbFunctionCallResultArgs,
    ReturnValue as FbReturnValue,
};

pub fn get_flatbuffer_result_from_int(value: i32) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let hlint = Fbhlint::create(&mut builder, &FbhlintArgs { value });

    let rt = FbReturnValue::hlint;
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hlint.as_union_value());

    get_flatbuffer_result(&mut builder, rt, rv)
}

pub fn get_flatbuffer_result_from_void() -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let hlvoid = Fbhlvoid::create(&mut builder, &FbhlvoidArgs {});

    let rt = FbReturnValue::hlvoid;
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hlvoid.as_union_value());

    get_flatbuffer_result(&mut builder, rt, rv)
}

pub fn get_flatbuffer_result_from_string(value: &str) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();

    let string_offset = builder.create_string(value);
    let hlstring = Fbhlstring::create(
        &mut builder,
        &FbhlstringArgs {
            value: Some(string_offset),
        },
    );

    let rt = FbReturnValue::hlstring;
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hlstring.as_union_value());

    get_flatbuffer_result(&mut builder, rt, rv)
}

/// # Safety
/// `value` could be a null pointer and we are dereferencing it.
pub unsafe fn get_flatbuffer_result_from_size_prefixed_buffer(
    value: *const u8,
    length: i32,
) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();

    let vec = unsafe { from_raw_parts(value, length as usize) };

    // Create a vector in the FlatBuffer using the data and length provided.
    let vec_offset = builder.create_vector(vec);

    let hlsizeprefixedbuffer = Fbhlsizeprefixedbuffer::create(
        &mut builder,
        &FbhlsizeprefixedbufferArgs {
            size_: length,
            value: Some(vec_offset),
        },
    );

    // Indicate that the return value is a size-prefixed buffer.
    let rt = FbReturnValue::hlsizeprefixedbuffer;
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hlsizeprefixedbuffer.as_union_value());

    // Get the FlatBuffer result.
    get_flatbuffer_result(&mut builder, rt, rv)
}

fn get_flatbuffer_result(
    builder: &mut FlatBufferBuilder,
    return_value_type: FbReturnValue,
    return_value: Option<WIPOffset<UnionWIPOffset>>,
) -> Vec<u8> {
    let result_offset = FbFunctionCallResult::create(
        builder,
        &FbFunctionCallResultArgs {
            return_value,
            return_value_type,
        },
    );

    builder.finish_size_prefixed(result_offset, None);

    builder.finished_data().to_vec()
}
