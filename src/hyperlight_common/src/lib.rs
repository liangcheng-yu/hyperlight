#![no_std]

extern crate alloc;

pub mod flatbuffer_wrappers;
/// FlatBuffers-related utilities and (mostly) generated code
#[allow(
    dead_code,
    unused_imports,
    clippy::all,
    unsafe_op_in_unsafe_fn,
    non_camel_case_types
)]
#[rustfmt::skip]
pub mod flatbuffers;

pub mod mem;
