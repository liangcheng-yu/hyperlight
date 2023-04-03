///! Data and functionality for representing and manipulating
///! arguments to guest and host functions.
pub mod args;
///! Definitions of both host and guest functions.
pub mod def;
///! Definitions for common functions to be exposed in the guest
pub(crate) mod exports;

use std::fmt::Debug;

/// SerializationType is the type of serialization
/// a given argument or return type has
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum SerializationType {
    // TODO: allow users to specify specific type of serialization
    // in this type. For example:
    // Raw(MsgPack)
    /// The absence of serialization. Payloads with this
    /// `SerializationType` should be treated as unserialized.
    Raw,
    /// The payload is serialized as a JSON string.
    Json,
    /// The payload is serialized with protocol buffers and should
    /// be deserialized as such.
    Proto,
}
