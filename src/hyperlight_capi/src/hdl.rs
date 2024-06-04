use super::handle::{Handle, Key, TypeID, EMPTY_KEY, INVALID_KEY, NULL_CONTEXT_KEY};
use hyperlight_host::{log_then_return, HyperlightError, Result};

/// The type-safe adapter to `Handle`
#[derive(Eq, Clone, PartialEq, Debug)]
pub(crate) enum Hdl {
    /// A reference to an `HyperlightError`
    Err(Key),
    /// A reference to a `bool`
    Boolean(Key),
    /// A reference to a `Sandbox`.
    Sandbox(Key),
    /// A reference to a nothing.
    ///
    /// Roughly equivalent to `NULL`.
    Empty(),
    /// A reference to an `Invalid` Handle error.
    ///
    /// Indicates that an invalid Handle was passed to the C api.
    Invalid(),
    /// A reference to a `NullContext` error.
    ///
    /// Indicates that a C api function was passed a null Context.
    NullContext(),
    /// A reference to a `String`.
    String(Key),
    /// A reference to a `Vec<u8>`.
    ByteArray(Key),
    /// A reference to a `SandboxMemoryManager`.
    MemMgr(Key),
    /// A reference to a `SharedMemory`.
    SharedMemory(Key),
    /// A reference to a `SharedMemorySnapshot`.
    SharedMemorySnapshot(Key),
    /// A reference to an `i64`
    Int64(Key),
    /// A reference to a `u64`
    UInt64(Key),
    /// A reference to an `i32`
    Int32(Key),
    /// A reference to a `u32`
    UInt32(Key),
    #[cfg(target_os = "linux")]
    /// A reference to a HyperV-on-linux hypervisor driver
    HypervLinuxDriver(Key),
    #[cfg(target_os = "linux")]
    /// A reference to a KVM hypervisor driver
    KVMDriver(Key),
    /// A reference to an outb handler function
    OutbHandlerFunc(Key),
    /// A reference to a memory access handler function
    MemAccessHandlerFunc(Key),
    /// A reference to a `GuestError`.
    GuestError(Key),
    /// A reference to a `FunctionCall` representing a HostFunctionCall.
    HostFunctionCall(Key),
    /// A reference to a `ReturnValue` representing a result from a function call.
    ReturnValue(Key),
    /// A reference to a `GuestLogData` representing data from a guest running
    /// in a VM sandbox
    GuestLogData(Key),
}

impl Hdl {
    const INVALID_TYPE_ID: TypeID = 1;
    const NULL_CONTEXT_TYPE_ID: TypeID = 2;
    const ERROR_TYPE_ID: TypeID = 100;
    const SANDBOX_TYPE_ID: TypeID = 101;
    const EMPTY_TYPE_ID: TypeID = 103;
    const STRING_TYPE_ID: TypeID = 105;
    const BYTE_ARRAY_TYPE_ID: TypeID = 106;
    const SHARED_MEMORY_TYPE_ID: TypeID = 110;
    const INT_64_TYPE_ID: TypeID = 111;
    const INT_32_TYPE_ID: TypeID = 112;
    #[cfg(target_os = "linux")]
    const HYPER_V_LINUX_DRIVER_TYPE_ID: TypeID = 120;
    const OUTB_HANDLER_FUNC_TYPE_ID: TypeID = 121;
    const MEM_ACCESS_HANDLER_FUNC_TYPE_ID: TypeID = 122;
    const SHARED_MEMORY_SNAPSHOT_TYPE_ID: TypeID = 123;
    const MEM_MGR_TYPE_ID: TypeID = 124;
    const UINT_64_TYPE_ID: TypeID = 125;
    const UINT_32_TYPE_ID: TypeID = 132;
    const BOOLEAN_TYPE_ID: TypeID = 126;
    const GUEST_ERROR_TYPE_ID: TypeID = 127;
    const HOST_FUNCTION_CALL_TYPE_ID: TypeID = 128;
    const RETURN_VALUE_TYPE_ID: TypeID = 129;
    const GUEST_LOG_DATA_TYPE_ID: TypeID = 130;
    #[cfg(target_os = "linux")]
    const KVM_DRIVER_TYPE_ID: TypeID = 131;

    /// Get the `TypeID` associated with `self`.
    ///
    /// This is often useful for interfacing with C APIs.
    pub(crate) fn type_id(&self) -> TypeID {
        match self {
            Hdl::Err(_) => Self::ERROR_TYPE_ID,
            Hdl::Boolean(_) => Self::BOOLEAN_TYPE_ID,
            Hdl::Sandbox(_) => Self::SANDBOX_TYPE_ID,
            Hdl::Empty() => Self::EMPTY_TYPE_ID,
            Hdl::Invalid() => Self::INVALID_TYPE_ID,
            Hdl::NullContext() => Self::NULL_CONTEXT_TYPE_ID,
            Hdl::String(_) => Self::STRING_TYPE_ID,
            Hdl::ByteArray(_) => Self::BYTE_ARRAY_TYPE_ID,
            Hdl::MemMgr(_) => Self::MEM_MGR_TYPE_ID,
            Hdl::SharedMemory(_) => Self::SHARED_MEMORY_TYPE_ID,
            Hdl::SharedMemorySnapshot(_) => Self::SHARED_MEMORY_SNAPSHOT_TYPE_ID,
            Hdl::Int64(_) => Self::INT_64_TYPE_ID,
            Hdl::UInt64(_) => Self::UINT_64_TYPE_ID,
            Hdl::Int32(_) => Self::INT_32_TYPE_ID,
            Hdl::UInt32(_) => Self::UINT_32_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::HypervLinuxDriver(_) => Self::HYPER_V_LINUX_DRIVER_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::KVMDriver(_) => Self::KVM_DRIVER_TYPE_ID,
            Hdl::OutbHandlerFunc(_) => Self::OUTB_HANDLER_FUNC_TYPE_ID,
            Hdl::MemAccessHandlerFunc(_) => Self::MEM_ACCESS_HANDLER_FUNC_TYPE_ID,
            Hdl::GuestError(_) => Self::GUEST_ERROR_TYPE_ID,
            Hdl::HostFunctionCall(_) => Self::HOST_FUNCTION_CALL_TYPE_ID,
            Hdl::ReturnValue(_) => Self::RETURN_VALUE_TYPE_ID,
            Hdl::GuestLogData(_) => Self::GUEST_LOG_DATA_TYPE_ID,
        }
    }

    /// Get the `Key` associated with `self`.
    ///
    /// This is useful for inserting, retrieving, and removing
    /// a given `Handle` from a `Context`.
    pub(crate) fn key(&self) -> Key {
        match self {
            Hdl::Err(key) => *key,
            Hdl::Boolean(key) => *key,
            Hdl::Sandbox(key) => *key,
            Hdl::Empty() => EMPTY_KEY,
            Hdl::Invalid() => INVALID_KEY,
            Hdl::NullContext() => NULL_CONTEXT_KEY,
            Hdl::String(key) => *key,
            Hdl::ByteArray(key) => *key,
            Hdl::MemMgr(key) => *key,
            Hdl::SharedMemory(key) => *key,
            Hdl::SharedMemorySnapshot(key) => *key,
            Hdl::Int64(key) => *key,
            Hdl::UInt64(key) => *key,
            Hdl::Int32(key) => *key,
            Hdl::UInt32(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::HypervLinuxDriver(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::KVMDriver(key) => *key,
            Hdl::OutbHandlerFunc(key) => *key,
            Hdl::MemAccessHandlerFunc(key) => *key,
            Hdl::GuestError(key) => *key,
            Hdl::HostFunctionCall(key) => *key,
            Hdl::ReturnValue(key) => *key,
            Hdl::GuestLogData(key) => *key,
        }
    }
}

impl std::fmt::Display for Hdl {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Hdl::Err(key) => write!(f, "Err({})", key),
            Hdl::Boolean(key) => write!(f, "Boolean({})", key),
            Hdl::Sandbox(key) => write!(f, "Sandbox({})", key),
            Hdl::Empty() => write!(f, "Empty()"),
            Hdl::Invalid() => write!(f, "Invalid()"),
            Hdl::NullContext() => write!(f, "NullContext()"),
            Hdl::String(key) => write!(f, "String({})", key),
            Hdl::ByteArray(key) => write!(f, "ByteArray({})", key),
            Hdl::MemMgr(key) => write!(f, "MemMgr({})", key),
            Hdl::SharedMemory(key) => write!(f, "SharedMemory({})", key),
            Hdl::SharedMemorySnapshot(key) => write!(f, "SharedMemorySnapshot({})", key),
            Hdl::Int64(key) => write!(f, "Int64({})", key),
            Hdl::UInt64(key) => write!(f, "UInt64({})", key),
            Hdl::Int32(key) => write!(f, "Int32({})", key),
            Hdl::UInt32(key) => write!(f, "UInt32({})", key),
            #[cfg(target_os = "linux")]
            Hdl::HypervLinuxDriver(key) => write!(f, "HypervLinuxDriver({})", key),
            #[cfg(target_os = "linux")]
            Hdl::KVMDriver(key) => write!(f, "KVMDriver({})", key),
            Hdl::OutbHandlerFunc(key) => write!(f, "OutbHandlerFunc({})", key),
            Hdl::MemAccessHandlerFunc(key) => write!(f, "MemAccessHandlerFunc({})", key),
            Hdl::GuestError(key) => write!(f, "GuestErrorHandlerFunc({})", key),
            Hdl::HostFunctionCall(key) => write!(f, "HostFunctionCallHandlerFunc({})", key),
            Hdl::ReturnValue(key) => write!(f, "ReturnValueHandlerFunc({})", key),
            Hdl::GuestLogData(key) => write!(f, "GuestLogData({})", key),
        }
    }
}

impl std::convert::TryFrom<Handle> for Hdl {
    type Error = HyperlightError;

    /// Create an instance of `Self` from `hdl` if `hdl` represents
    /// a valid `Handle`.
    fn try_from(hdl: Handle) -> Result<Self> {
        let key = hdl.key();
        match hdl.type_id() {
            Self::ERROR_TYPE_ID => Ok(Hdl::Err(key)),
            Self::BOOLEAN_TYPE_ID => Ok(Hdl::Boolean(key)),
            Self::SANDBOX_TYPE_ID => Ok(Hdl::Sandbox(key)),
            Self::EMPTY_TYPE_ID => Ok(Hdl::Empty()),
            Self::INVALID_TYPE_ID => Ok(Hdl::Invalid()),
            Self::NULL_CONTEXT_TYPE_ID => Ok(Hdl::NullContext()),
            Self::STRING_TYPE_ID => Ok(Hdl::String(key)),
            Self::BYTE_ARRAY_TYPE_ID => Ok(Hdl::ByteArray(key)),
            Self::MEM_MGR_TYPE_ID => Ok(Hdl::MemMgr(key)),
            Self::SHARED_MEMORY_TYPE_ID => Ok(Hdl::SharedMemory(key)),
            Self::SHARED_MEMORY_SNAPSHOT_TYPE_ID => Ok(Hdl::SharedMemorySnapshot(key)),
            Self::INT_64_TYPE_ID => Ok(Hdl::Int64(key)),
            Self::UINT_64_TYPE_ID => Ok(Hdl::UInt64(key)),
            Self::INT_32_TYPE_ID => Ok(Hdl::Int32(key)),
            Self::UINT_32_TYPE_ID => Ok(Hdl::UInt32(key)),
            #[cfg(target_os = "linux")]
            Self::HYPER_V_LINUX_DRIVER_TYPE_ID => Ok(Hdl::HypervLinuxDriver(key)),
            #[cfg(target_os = "linux")]
            Self::KVM_DRIVER_TYPE_ID => Ok(Hdl::KVMDriver(key)),
            Self::OUTB_HANDLER_FUNC_TYPE_ID => Ok(Hdl::OutbHandlerFunc(key)),
            Self::MEM_ACCESS_HANDLER_FUNC_TYPE_ID => Ok(Hdl::MemAccessHandlerFunc(key)),
            Self::GUEST_ERROR_TYPE_ID => Ok(Hdl::GuestError(key)),
            Self::HOST_FUNCTION_CALL_TYPE_ID => Ok(Hdl::HostFunctionCall(key)),
            Self::RETURN_VALUE_TYPE_ID => Ok(Hdl::ReturnValue(key)),
            Self::GUEST_LOG_DATA_TYPE_ID => Ok(Hdl::GuestLogData(key)),
            _ => {
                log_then_return!("invalid handle type {}", hdl.type_id());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::handle::{new_key, Handle};
    use super::Hdl;
    use hyperlight_host::Result;

    #[test]
    fn handle_type_id() -> Result<()> {
        let key = new_key();
        let handle = Handle::from(Hdl::Sandbox(key));
        assert_eq!(handle.type_id(), Hdl::SANDBOX_TYPE_ID);
        Ok(())
    }
}
