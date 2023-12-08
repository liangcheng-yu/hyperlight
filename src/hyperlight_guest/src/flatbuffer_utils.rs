use alloc::vec::Vec;
use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use hyperlight_flatbuffers::flatbuffers::hyperlight::generated::{
    hlint as Fbhlint, hlintArgs as FbhlintArgs, hlvoid as Fbhlvoid, hlvoidArgs as FbhlvoidArgs,
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
