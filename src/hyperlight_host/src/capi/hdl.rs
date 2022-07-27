use super::handle::{Handle, Key, TypeID, EMPTY_KEY};
use anyhow::bail;

/// The type-safe adapter to `Handle`
#[derive(Eq, Clone, PartialEq, Debug)]
pub enum Hdl {
    Err(Key),
    Sandbox(Key),
    Val(Key),
    Empty(),
    HostFunc(Key),
    String(Key),
    ByteArray(Key),
    PEInfo(Key),
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
        }
    }
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
