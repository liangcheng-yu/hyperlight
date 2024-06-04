use alloc::vec::Vec;
use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use hyperlight_common::flatbuffers::hyperlight::generated::{
    hlint as Fbhlint, hlintArgs as FbhlintArgs, hllong as Fbhllong, hllongArgs as FbhllongArgs,
    hlsizeprefixedbuffer as Fbhlsizeprefixedbuffer,
    hlsizeprefixedbufferArgs as FbhlsizeprefixedbufferArgs, hlstring as Fbhlstring,
    hlstringArgs as FbhlstringArgs, hluint as Fbhluint, hluintArgs as FbhluintArgs,
    hlulong as Fbhlulong, hlulongArgs as FbhlulongArgs, hlvoid as Fbhlvoid,
    hlvoidArgs as FbhlvoidArgs, FunctionCallResult as FbFunctionCallResult,
    FunctionCallResultArgs as FbFunctionCallResultArgs, ReturnValue as FbReturnValue,
};

pub fn get_flatbuffer_result_from_int(value: i32) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let hlint = Fbhlint::create(&mut builder, &FbhlintArgs { value });

    let rt = FbReturnValue::hlint;
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hlint.as_union_value());

    get_flatbuffer_result(&mut builder, rt, rv)
}

pub fn get_flatbuffer_result_from_uint(value: u32) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let hluint = Fbhluint::create(&mut builder, &FbhluintArgs { value });

    let rt = FbReturnValue::hluint;
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hluint.as_union_value());

    get_flatbuffer_result(&mut builder, rt, rv)
}

pub fn get_flatbuffer_result_from_long(value: i64) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let hllong = Fbhllong::create(&mut builder, &FbhllongArgs { value });

    let rt = FbReturnValue::hllong;
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hllong.as_union_value());

    get_flatbuffer_result(&mut builder, rt, rv)
}

pub fn get_flatbuffer_result_from_ulong(value: u64) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let hlulong = Fbhlulong::create(&mut builder, &FbhlulongArgs { value });

    let rt = FbReturnValue::hlulong;
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hlulong.as_union_value());

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

pub fn get_flatbuffer_result_from_vec(data: &[u8]) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();

    let vec_offset = builder.create_vector(data);

    let hlsizeprefixedbuffer = Fbhlsizeprefixedbuffer::create(
        &mut builder,
        &FbhlsizeprefixedbufferArgs {
            size_: data.len() as i32,
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
