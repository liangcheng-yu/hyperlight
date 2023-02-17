use super::handle::{Handle, Key, TypeID, EMPTY_KEY, INVALID_KEY, NULL_CONTEXT_KEY};
use anyhow::bail;

/// The type-safe adapter to `Handle`
#[derive(Eq, Clone, PartialEq, Debug)]
pub enum Hdl {
    /// A reference to an `anyhow::Error`
    Err(Key),
    /// A reference to a `bool`
    Boolean(Key),
    /// A reference to a `Sandbox`.
    Sandbox(Key),
    /// A reference to a `Val`.
    Val(Key),
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
    /// A reference to a `HostFunc`.
    HostFunc(Key),
    /// A reference to a `String`.
    String(Key),
    /// A reference to a `Vec<u8>`.
    ByteArray(Key),
    /// A reference to a `PEInfo`.
    PEInfo(Key),
    /// A reference to a `SandboxMemoryLayout`.
    MemLayout(Key),
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
    #[cfg(target_os = "linux")]
    /// A reference to a KVM instance
    Kvm(Key),
    #[cfg(target_os = "linux")]
    /// A reference to a KVM VmFd instance
    KvmVmFd(Key),
    #[cfg(target_os = "linux")]
    /// A reference to a KVM VcpuFd instance
    KvmVcpuFd(Key),
    #[cfg(target_os = "linux")]
    /// A reference to a KVM `kvm_userspace_memory_region` instance
    KvmUserMemRegion(Key),
    #[cfg(target_os = "linux")]
    /// A reference to a KVM run message instance
    KvmRunMessage(Key),
    #[cfg(target_os = "linux")]
    /// A reference to the KVM registers
    KvmRegisters(Key),
    #[cfg(target_os = "linux")]
    /// A reference to the KVM segment registers
    KvmSRegisters(Key),
    #[cfg(target_os = "linux")]
    /// A reference to a HyperV linux driver
    HypervLinuxDriver(Key),
    /// A reference to an outb handler function
    OutbHandlerFunc(Key),
    /// A reference to a memory access handler function
    MemAccessHandlerFunc(Key),
    /// A reference to a `GuestError`.
    GuestError(Key),
}

impl Hdl {
    const INVALID_TYPE_ID: TypeID = 1;
    const NULL_CONTEXT_TYPE_ID: TypeID = 2;
    const ERROR_TYPE_ID: TypeID = 100;
    const SANDBOX_TYPE_ID: TypeID = 101;
    const VAL_TYPE_ID: TypeID = 102;
    const EMPTY_TYPE_ID: TypeID = 103;
    const HOST_FUNC_TYPE_ID: TypeID = 104;
    const STRING_TYPE_ID: TypeID = 105;
    const BYTE_ARRAY_TYPE_ID: TypeID = 106;
    const PE_INFO_TYPE_ID: TypeID = 107;
    const MEM_LAYOUT_TYPE_ID: TypeID = 109;
    const SHARED_MEMORY_TYPE_ID: TypeID = 110;
    const INT_64_TYPE_ID: TypeID = 111;
    const INT_32_TYPE_ID: TypeID = 112;
    #[cfg(target_os = "linux")]
    const KVM_TYPE_ID: TypeID = 113;
    #[cfg(target_os = "linux")]
    const KVM_VMFD_TYPE_ID: TypeID = 114;
    #[cfg(target_os = "linux")]
    const KVM_VCPUFD_TYPE_ID: TypeID = 115;
    #[cfg(target_os = "linux")]
    const KVM_USER_MEM_REGION_TYPE_ID: TypeID = 116;
    #[cfg(target_os = "linux")]
    const KVM_RUN_MESSAGE_TYPE_ID: TypeID = 117;
    #[cfg(target_os = "linux")]
    const KVM_REGISTERS_TYPE_ID: TypeID = 118;
    #[cfg(target_os = "linux")]
    const KVM_SREGISTERS_TYPE_ID: TypeID = 119;
    #[cfg(target_os = "linux")]
    const HYPER_V_LINUX_DRIVER_TYPE_ID: TypeID = 120;
    const OUTB_HANDLER_FUNC_TYPE_ID: TypeID = 121;
    const MEM_ACCESS_HANDLER_FUNC_TYPE_ID: TypeID = 122;
    const SHARED_MEMORY_SNAPSHOT_TYPE_ID: TypeID = 123;
    const MEM_MGR_TYPE_ID: TypeID = 124;
    const UINT_64_TYPE_ID: TypeID = 125;
    const BOOLEAN_TYPE_ID: TypeID = 126;
    const GUEST_ERROR_TYPE_ID: TypeID = 127;

    /// Get the `TypeID` associated with `self`.
    ///
    /// This is often useful for interfacing with C APIs.
    pub fn type_id(&self) -> TypeID {
        match self {
            Hdl::Err(_) => Self::ERROR_TYPE_ID,
            Hdl::Boolean(_) => Self::BOOLEAN_TYPE_ID,
            Hdl::Sandbox(_) => Self::SANDBOX_TYPE_ID,
            Hdl::Val(_) => Self::VAL_TYPE_ID,
            Hdl::Empty() => Self::EMPTY_TYPE_ID,
            Hdl::Invalid() => Self::INVALID_TYPE_ID,
            Hdl::NullContext() => Self::NULL_CONTEXT_TYPE_ID,
            Hdl::HostFunc(_) => Self::HOST_FUNC_TYPE_ID,
            Hdl::String(_) => Self::STRING_TYPE_ID,
            Hdl::ByteArray(_) => Self::BYTE_ARRAY_TYPE_ID,
            Hdl::PEInfo(_) => Self::PE_INFO_TYPE_ID,
            Hdl::MemLayout(_) => Self::MEM_LAYOUT_TYPE_ID,
            Hdl::MemMgr(_) => Self::MEM_MGR_TYPE_ID,
            Hdl::SharedMemory(_) => Self::SHARED_MEMORY_TYPE_ID,
            Hdl::SharedMemorySnapshot(_) => Self::SHARED_MEMORY_SNAPSHOT_TYPE_ID,
            Hdl::Int64(_) => Self::INT_64_TYPE_ID,
            Hdl::UInt64(_) => Self::UINT_64_TYPE_ID,
            Hdl::Int32(_) => Self::INT_32_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::Kvm(_) => Self::KVM_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::KvmVmFd(_) => Self::KVM_VMFD_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::KvmVcpuFd(_) => Self::KVM_VCPUFD_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::KvmUserMemRegion(_) => Self::KVM_USER_MEM_REGION_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::KvmRunMessage(_) => Self::KVM_RUN_MESSAGE_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::KvmRegisters(_) => Self::KVM_REGISTERS_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::KvmSRegisters(_) => Self::KVM_SREGISTERS_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::HypervLinuxDriver(_) => Self::HYPER_V_LINUX_DRIVER_TYPE_ID,
            Hdl::OutbHandlerFunc(_) => Self::OUTB_HANDLER_FUNC_TYPE_ID,
            Hdl::MemAccessHandlerFunc(_) => Self::MEM_ACCESS_HANDLER_FUNC_TYPE_ID,
            Hdl::GuestError(_) => Self::GUEST_ERROR_TYPE_ID,
        }
    }

    /// Get the `Key` associated with `self`.
    ///
    /// This is useful for inserting, retrieving, and removing
    /// a given `Handle` from a `Context`.
    pub fn key(&self) -> Key {
        match self {
            Hdl::Err(key) => *key,
            Hdl::Boolean(key) => *key,
            Hdl::Sandbox(key) => *key,
            Hdl::Val(key) => *key,
            Hdl::Empty() => EMPTY_KEY,
            Hdl::Invalid() => INVALID_KEY,
            Hdl::NullContext() => NULL_CONTEXT_KEY,
            Hdl::HostFunc(key) => *key,
            Hdl::String(key) => *key,
            Hdl::ByteArray(key) => *key,
            Hdl::PEInfo(key) => *key,
            Hdl::MemLayout(key) => *key,
            Hdl::MemMgr(key) => *key,
            Hdl::SharedMemory(key) => *key,
            Hdl::SharedMemorySnapshot(key) => *key,
            Hdl::Int64(key) => *key,
            Hdl::UInt64(key) => *key,
            Hdl::Int32(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::Kvm(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::KvmVmFd(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::KvmVcpuFd(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::KvmUserMemRegion(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::KvmRunMessage(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::KvmRegisters(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::KvmSRegisters(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::HypervLinuxDriver(key) => *key,
            Hdl::OutbHandlerFunc(key) => *key,
            Hdl::MemAccessHandlerFunc(key) => *key,
            Hdl::GuestError(key) => *key,
        }
    }
}

impl std::fmt::Display for Hdl {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Hdl::Err(key) => write!(f, "Err({})", key),
            Hdl::Boolean(key) => write!(f, "Boolean({})", key),
            Hdl::Sandbox(key) => write!(f, "Sandbox({})", key),
            Hdl::Val(key) => write!(f, "Val({})", key),
            Hdl::Empty() => write!(f, "Empty()"),
            Hdl::Invalid() => write!(f, "Invalid()"),
            Hdl::NullContext() => write!(f, "NullContext()"),
            Hdl::HostFunc(key) => write!(f, "HostFunc({})", key),
            Hdl::String(key) => write!(f, "String({})", key),
            Hdl::ByteArray(key) => write!(f, "ByteArray({})", key),
            Hdl::PEInfo(key) => write!(f, "PEInfo({})", key),
            Hdl::MemLayout(key) => write!(f, "MemLayout({})", key),
            Hdl::MemMgr(key) => write!(f, "MemMgr({})", key),
            Hdl::SharedMemory(key) => write!(f, "SharedMemory({})", key),
            Hdl::SharedMemorySnapshot(key) => write!(f, "SharedMemorySnapshot({})", key),
            Hdl::Int64(key) => write!(f, "Int64({})", key),
            Hdl::UInt64(key) => write!(f, "UInt64({})", key),
            Hdl::Int32(key) => write!(f, "Int32({})", key),
            #[cfg(target_os = "linux")]
            Hdl::Kvm(key) => write!(f, "Kvm({})", key),
            #[cfg(target_os = "linux")]
            Hdl::KvmVmFd(key) => write!(f, "KvmVmFd({})", key),
            #[cfg(target_os = "linux")]
            Hdl::KvmVcpuFd(key) => write!(f, "KvmVcpuFd({})", key),
            #[cfg(target_os = "linux")]
            Hdl::KvmUserMemRegion(key) => write!(f, "KvmUserMemRegion({})", key),
            #[cfg(target_os = "linux")]
            Hdl::KvmRunMessage(key) => write!(f, "KvmRunMessage({})", key),
            #[cfg(target_os = "linux")]
            Hdl::KvmRegisters(key) => write!(f, "KvmRegisters({})", key),
            #[cfg(target_os = "linux")]
            Hdl::KvmSRegisters(key) => write!(f, "KvmSRegisters({})", key),
            #[cfg(target_os = "linux")]
            Hdl::HypervLinuxDriver(key) => write!(f, "HypervLinuxDriver({})", key),
            Hdl::OutbHandlerFunc(key) => write!(f, "OutbHandlerFunc({})", key),
            Hdl::MemAccessHandlerFunc(key) => write!(f, "MemAccessHandlerFunc({})", key),
            Hdl::GuestError(key) => write!(f, "GuestErrorHandlerFunc({})", key),
        }
    }
}

impl std::convert::TryFrom<Handle> for Hdl {
    type Error = anyhow::Error;

    /// Create an instance of `Self` from `hdl` if `hdl` represents
    /// a valid `Handle`.
    fn try_from(hdl: Handle) -> anyhow::Result<Self> {
        let key = hdl.key();
        match hdl.type_id() {
            Self::ERROR_TYPE_ID => Ok(Hdl::Err(key)),
            Self::BOOLEAN_TYPE_ID => Ok(Hdl::Boolean(key)),
            Self::SANDBOX_TYPE_ID => Ok(Hdl::Sandbox(key)),
            Self::VAL_TYPE_ID => Ok(Hdl::Val(key)),
            Self::EMPTY_TYPE_ID => Ok(Hdl::Empty()),
            Self::INVALID_TYPE_ID => Ok(Hdl::Invalid()),
            Self::NULL_CONTEXT_TYPE_ID => Ok(Hdl::NullContext()),
            Self::HOST_FUNC_TYPE_ID => Ok(Hdl::HostFunc(key)),
            Self::STRING_TYPE_ID => Ok(Hdl::String(key)),
            Self::BYTE_ARRAY_TYPE_ID => Ok(Hdl::ByteArray(key)),
            Self::PE_INFO_TYPE_ID => Ok(Hdl::PEInfo(key)),
            Self::MEM_LAYOUT_TYPE_ID => Ok(Hdl::MemLayout(key)),
            Self::MEM_MGR_TYPE_ID => Ok(Hdl::MemMgr(key)),
            Self::SHARED_MEMORY_TYPE_ID => Ok(Hdl::SharedMemory(key)),
            Self::SHARED_MEMORY_SNAPSHOT_TYPE_ID => Ok(Hdl::SharedMemorySnapshot(key)),
            Self::INT_64_TYPE_ID => Ok(Hdl::Int64(key)),
            Self::UINT_64_TYPE_ID => Ok(Hdl::UInt64(key)),
            Self::INT_32_TYPE_ID => Ok(Hdl::Int32(key)),
            #[cfg(target_os = "linux")]
            Self::KVM_TYPE_ID => Ok(Hdl::Kvm(key)),
            #[cfg(target_os = "linux")]
            Self::KVM_VMFD_TYPE_ID => Ok(Hdl::KvmVmFd(key)),
            #[cfg(target_os = "linux")]
            Self::KVM_VCPUFD_TYPE_ID => Ok(Hdl::KvmVcpuFd(key)),
            #[cfg(target_os = "linux")]
            Self::KVM_USER_MEM_REGION_TYPE_ID => Ok(Hdl::KvmUserMemRegion(key)),
            #[cfg(target_os = "linux")]
            Self::KVM_RUN_MESSAGE_TYPE_ID => Ok(Hdl::KvmRunMessage(key)),
            #[cfg(target_os = "linux")]
            Self::KVM_REGISTERS_TYPE_ID => Ok(Hdl::KvmRegisters(key)),
            #[cfg(target_os = "linux")]
            Self::KVM_SREGISTERS_TYPE_ID => Ok(Hdl::KvmSRegisters(key)),
            #[cfg(target_os = "linux")]
            Self::HYPER_V_LINUX_DRIVER_TYPE_ID => Ok(Hdl::HypervLinuxDriver(key)),
            Self::OUTB_HANDLER_FUNC_TYPE_ID => Ok(Hdl::OutbHandlerFunc(key)),
            Self::MEM_ACCESS_HANDLER_FUNC_TYPE_ID => Ok(Hdl::MemAccessHandlerFunc(key)),
            Self::GUEST_ERROR_TYPE_ID => Ok(Hdl::GuestError(key)),
            _ => bail!("invalid handle type {}", hdl.type_id()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::handle::{new_key, Handle};
    use super::Hdl;
    use anyhow::Result;

    #[test]
    fn handle_type_id() -> Result<()> {
        let key = new_key();
        let handle = Handle::from(Hdl::Sandbox(key));
        assert_eq!(handle.type_id(), Hdl::SANDBOX_TYPE_ID);
        Ok(())
    }
}
