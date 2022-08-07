use super::handle::{Handle, Key, TypeID, EMPTY_KEY};
use anyhow::bail;

/// The type-safe adapter to `Handle`
#[derive(Eq, Clone, PartialEq, Debug)]
pub enum Hdl {
    /// A reference to an `anyhow::Error`
    Err(Key),
    /// A reference to a `Sandbox`.
    Sandbox(Key),
    /// A reference to a `Val`.
    Val(Key),
    /// A reference to a nothing.
    ///
    /// Roughly equivalent to `NULL`.
    Empty(),
    /// A reference to a `HostFunc`.
    HostFunc(Key),
    /// A reference to a `String`.
    String(Key),
    /// A reference to a `Vec<u8>`.
    ByteArray(Key),
    /// A reference to a `PEInfo`.
    PEInfo(Key),
    /// A reference to a `SandboxMemoryConfiguration`.
    MemConfig(Key),
    /// A reference to a `SandboxMemoryLayout`.
    MemLayout(Key),
    #[cfg(target_os = "linux")]
    Mshv(Key),
    #[cfg(target_os = "linux")]
    VmFd(Key),
    #[cfg(target_os = "linux")]
    VcpuFd(Key),
    #[cfg(target_os = "linux")]
    MshvUserMemRegion(Key),
    #[cfg(target_os = "linux")]
    MshvRunMessage(Key),
}

impl Hdl {
    const ERROR_TYPE_ID: TypeID = 100;
    const SANDBOX_TYPE_ID: TypeID = 101;
    const VAL_TYPE_ID: TypeID = 102;
    const EMPTY_TYPE_ID: TypeID = 103;
    const HOST_FUNC_TYPE_ID: TypeID = 104;
    const STRING_TYPE_ID: TypeID = 105;
    const BYTE_ARRAY_TYPE_ID: TypeID = 106;
    const PE_INFO_TYPE_ID: TypeID = 107;
    const MEM_CONFIG_TYPE_ID: TypeID = 108;
    const MEM_LAYOUT_TYPE_ID: TypeID = 109;
    #[cfg(target_os = "linux")]
    const MSHV_TYPE_ID: TypeID = 110;
    #[cfg(target_os = "linux")]
    const VM_FD_TYPE_ID: TypeID = 111;
    #[cfg(target_os = "linux")]
    const VCPU_FD_TYPE_ID: TypeID = 112;
    #[cfg(target_os = "linux")]
    const MSHV_USER_MEM_REGION_TYPE_ID: TypeID = 113;
    #[cfg(target_os = "linux")]
    const MSHV_RUN_MESSAGE_TYPE_ID: TypeID = 114;

    /// Get the `TypeID` associated with `self`.
    ///
    /// This is often useful for interfacing with C APIs.
    pub fn type_id(&self) -> TypeID {
        match self {
            Hdl::Err(_) => Self::ERROR_TYPE_ID,
            Hdl::Sandbox(_) => Self::SANDBOX_TYPE_ID,
            Hdl::Val(_) => Self::VAL_TYPE_ID,
            Hdl::Empty() => Self::EMPTY_TYPE_ID,
            Hdl::HostFunc(_) => Self::HOST_FUNC_TYPE_ID,
            Hdl::String(_) => Self::STRING_TYPE_ID,
            Hdl::ByteArray(_) => Self::BYTE_ARRAY_TYPE_ID,
            Hdl::PEInfo(_) => Self::PE_INFO_TYPE_ID,
            Hdl::MemConfig(_) => Self::MEM_CONFIG_TYPE_ID,
            Hdl::MemLayout(_) => Self::MEM_LAYOUT_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::Mshv(_) => Self::MSHV_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::VmFd(_) => Self::VM_FD_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::VcpuFd(_) => Self::VCP_FD_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::MshvUserMemRegion(_) => Self::MSHV_USER_MEM_REGION_TYPE_ID,
            #[cfg(target_os = "linux")]
            Hdl::MshvRunMessage(_) => Self::MSHV_RUN_MESSAGE_TYPE_ID,
        }
    }

    /// Get the `Key` associated with `self`.
    ///
    /// This is useful for inserting, retrieving, and removing
    /// a given `Handle` from a `Context`.
    pub fn key(&self) -> Key {
        match self {
            Hdl::Err(key) => *key,
            Hdl::Sandbox(key) => *key,
            Hdl::Val(key) => *key,
            Hdl::Empty() => EMPTY_KEY,
            Hdl::HostFunc(key) => *key,
            Hdl::String(key) => *key,
            Hdl::ByteArray(key) => *key,
            Hdl::PEInfo(key) => *key,
            Hdl::MemConfig(key) => *key,
            Hdl::MemLayout(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::Mshv(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::VmFd(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::VcpuFd(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::MshvUserMemRegion(key) => *key,
            #[cfg(target_os = "linux")]
            Hdl::MshvRunMessage(key) => *key,
        }
    }
}

impl std::fmt::Display for Hdl {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Hdl::Err(key) => write!(f, "Err({})", key),
            Hdl::Sandbox(key) => write!(f, "Sandbox({})", key),
            Hdl::Val(key) => write!(f, "Val({})", key),
            Hdl::Empty() => write!(f, "Empty()"),
            Hdl::HostFunc(key) => write!(f, "HostFunc({})", key),
            Hdl::String(key) => write!(f, "String({})", key),
            Hdl::ByteArray(key) => write!(f, "ByteArray({})", key),
            Hdl::PEInfo(key) => write!(f, "PEInfo({})", key),
            Hdl::MemConfig(key) => write!(f, "MemConfig({})", key),
            Hdl::MemLayout(key) => write!(f, "MemLayout({})", key),
            #[cfg(target_os = "linux")]
            Hdl::Mshv(key) => write!(f, "Mshv({})", key),
            #[cfg(target_os = "linux")]
            Hdl::VmFd(key) => write!(f, "VmFd({})", key),
            #[cfg(target_os = "linux")]
            Hdl::VcpuFd(key) => write!(f, "VcpuFd({})", key),
            #[cfg(target_os = "linux")]
            Hdl::MshvUserMemRegion(key) => write!(f, "MshvUserMemRegion({})", key),
            #[cfg(target_os = "linux")]
            Hdl::MshvRunMessage(key) => write!(f, "MshvRunMessage({})", key),
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
            Self::SANDBOX_TYPE_ID => Ok(Hdl::Sandbox(key)),
            Self::VAL_TYPE_ID => Ok(Hdl::Val(key)),
            Self::EMPTY_TYPE_ID => Ok(Hdl::Empty()),
            Self::HOST_FUNC_TYPE_ID => Ok(Hdl::HostFunc(key)),
            Self::STRING_TYPE_ID => Ok(Hdl::String(key)),
            Self::BYTE_ARRAY_TYPE_ID => Ok(Hdl::ByteArray(key)),
            Self::PE_INFO_TYPE_ID => Ok(Hdl::PEInfo(key)),
            Self::MEM_CONFIG_TYPE_ID => Ok(Hdl::MemConfig(key)),
            Self::MEM_LAYOUT_TYPE_ID => Ok(Hdl::MemLayout(key)),
            #[cfg(target_os = "linux")]
            Self::MSHV_TYPE_ID => Ok(Hdl::Mshv(key)),
            #[cfg(target_os = "linux")]
            Self::VM_FD_TYPE_ID => Ok(Hdl::VmFd(key)),
            #[cfg(target_os = "linux")]
            Self::VCPU_FD_TYPE_ID => Ok(Hdl::VcpuFd(key)),
            #[cfg(target_os = "linux")]
            Self::MSHV_USER_MEM_REGION_TYPE_ID => Ok(Hdl::MshvUserMemRegion(key)),
            #[cfg(target_os = "linux")]
            Self::MSHV_RUN_MESSAGE_TYPE_ID => Ok(Hdl::MshvRunMessage(key)),
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
