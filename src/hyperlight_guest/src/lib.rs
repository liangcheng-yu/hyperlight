#![no_std]

pub mod hyperlight_peb;
pub mod guest;
#[allow(non_camel_case_types)]
pub mod flatbuffers;

// links memset, memcpy, __CxxFrameHandler3, and memcmp
#[link(name = "vcruntime")]
extern {}

#[link(name = "ucrt")]
extern {}

// links _fltused symbol
#[link(name = "msvcrt")]
extern {}