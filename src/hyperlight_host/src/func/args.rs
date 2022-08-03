use super::SerializationType;
use std::any::TypeId;
use std::fmt::Debug;
use std::vec::Vec;

/// ValType is the type of either an argument to, or return of, a function
#[derive(Debug)]
pub struct ValType {
    _type_id: TypeId,
}

/// Val is an argument to, or return type from, a function
/// that will be called across the VM boundary.
/// That is, either the host will call the guest or
/// vice-versa
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Val {
    /// The data being passed in this `Val`.
    ///
    /// This `Vec` should generally not be modified or read and
    /// treated as opaque by most parts of the system. Generally
    /// speaking, the only code that should attempt to read or
    /// write it should also be code responsible for (de)serializing
    /// it.
    pub data: Vec<i8>,
    /// The method with which `data` was serialized and thus can
    /// be deserialized.
    pub ser_type: SerializationType,
}

impl Val {
    /// Create a new Val with the given data and serialization type
    pub fn new(data: Vec<i8>, ser_type: SerializationType) -> Self {
        Self { data, ser_type }
    }
}
