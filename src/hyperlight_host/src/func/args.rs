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
    pub data: Vec<i8>,
    pub ser_type: SerializationType,
}

impl Val {
    /// Create a new Val with the given data and serialization type
    pub fn new(data: Vec<i8>, ser_type: SerializationType) -> Self {
        Self { data, ser_type }
    }
}
