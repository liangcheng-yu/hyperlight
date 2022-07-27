pub mod args;
pub mod def;

use std::fmt::Debug;

/// SerializationType is the type of serialization
/// a given argument or return type has
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum SerializationType {
    // TODO: allow users to specify specific type of serialization
    // in this type. For example:
    // Raw(MsgPack)
    Raw,
    Json,
    Proto,
}
