use super::handle::{new_key, Handle, Key};
use super::hdl::Hdl;
use crate::mem::pe::PEInfo;
use crate::sandbox::Sandbox;
use crate::{func::args::Val, mem::config::SandboxMemoryConfiguration};
use crate::{func::def::HostFunc, mem::layout::SandboxMemoryLayout};
use anyhow::{bail, Error, Result};
use chashmap::{CHashMap, ReadGuard, WriteGuard};

#[derive(Default)]
pub struct Context {
    pub errs: CHashMap<Key, Error>,
    pub sandboxes: CHashMap<Key, Sandbox>,
    pub vals: CHashMap<Key, Val>,
    pub host_funcs: CHashMap<Key, HostFunc>,
    pub strings: CHashMap<Key, String>,
    pub byte_arrays: CHashMap<Key, Vec<u8>>,
    pub pe_infos: CHashMap<Key, PEInfo>,
    pub mem_configs: CHashMap<Key, SandboxMemoryConfiguration>,
    pub mem_layouts: CHashMap<Key, SandboxMemoryLayout>,
}

pub type ReadResult<'a, T> = Result<ReadGuard<'a, Key, T>>;
pub type WriteResult<'a, T> = Result<WriteGuard<'a, Key, T>>;

impl Context {
    pub fn register<T, HandleFn: FnOnce(Key) -> Hdl>(
        obj: T,
        coll: &CHashMap<Key, T>,
        make_handle: HandleFn,
    ) -> Handle {
        let key = new_key();
        let handle = Handle::from(make_handle(key));
        coll.insert(handle.key(), obj);
        handle
    }

    pub fn register_err(&mut self, err: Error) -> Handle {
        Self::register(err, &self.errs, Hdl::Err)
    }

    pub fn get<T, ChkFn: FnOnce(&Hdl) -> bool>(
        handle: Handle,
        coll: &CHashMap<Key, T>,
        chk: ChkFn,
    ) -> ReadResult<T> {
        let hdl = Hdl::try_from(handle)?;
        if !chk(&hdl) {
            bail!("invalid handle")
        }
        match coll.get(&handle.key()) {
            Some(obj) => Ok(obj),
            None => bail!("object {} not found for key {}", hdl, handle.key()),
        }
    }

    pub fn get_mut<T, ChkFn: FnOnce(&Hdl) -> bool>(
        handle: Handle,
        coll: &CHashMap<Key, T>,
        chk: ChkFn,
    ) -> WriteResult<T> {
        let hdl = Hdl::try_from(handle)?;
        if !chk(&hdl) {
            bail!("invalid handle")
        }
        match coll.get_mut(&handle.key()) {
            Some(obj) => Ok(obj),
            None => bail!("object {} not found for key {}", hdl, handle.key()),
        }
    }

    pub fn remove<ChkFn: FnOnce(&Hdl) -> bool>(&mut self, handle: Handle, chk_fn: ChkFn) -> bool {
        match Hdl::try_from(handle) {
            Ok(hdl) => {
                if !chk_fn(&hdl) {
                    return false;
                }
                match hdl {
                    Hdl::Err(key) => self.errs.remove(&key).is_some(),
                    Hdl::Sandbox(key) => self.sandboxes.remove(&key).is_some(),
                    Hdl::Empty() => true,
                    Hdl::Val(key) => self.vals.remove(&key).is_some(),
                    Hdl::HostFunc(key) => self.host_funcs.remove(&key).is_some(),
                    Hdl::String(key) => self.strings.remove(&key).is_some(),
                    Hdl::ByteArray(key) => self.byte_arrays.remove(&key).is_some(),
                    Hdl::PEInfo(key) => self.pe_infos.remove(&key).is_some(),
                    Hdl::MemConfig(key) => self.mem_configs.remove(&key).is_some(),
                    Hdl::MemLayout(key) => self.mem_layouts.remove(&key).is_some(),
                }
            }
            Err(_) => false,
        }
    }
}

/// Create a new context for use in the C API.
#[no_mangle]
pub extern "C" fn context_new() -> *mut Context {
    Box::into_raw(Box::new(Context::default()))
}

/// Free the memory referenced by with `ctx`.
///
/// # Safety
///
/// You must only call this function:
///
/// - Exactly once per `ctx` parameter
/// - Only after a given `ctx` is done being used
/// - With `Context`s created by `context_new`
#[no_mangle]
pub unsafe extern "C" fn context_free(ctx: *mut Context) {
    Box::from_raw(ctx);
}

#[cfg(test)]
mod tests {
    use super::Context;
    use crate::capi::byte_array::get_byte_array_mut;
    use crate::capi::hdl::Hdl;
    use crate::capi::strings::get_string;
    use crate::capi::val_ref::get_val;
    use crate::func::args::Val;
    use crate::func::SerializationType;
    use anyhow::Result;

    #[test]
    fn round_trip_string() -> Result<()> {
        let ctx = Context::default();
        let start = "hello".to_string();
        let hdl_res = Context::register(start, &ctx.strings, Hdl::String);
        Context::get(hdl_res, &ctx.strings, |s| matches!(s, Hdl::String(_)))?;
        Ok(())
    }

    #[test]
    fn round_trip_val() -> Result<()> {
        let ctx = Context::default();
        let start = Val::new(Vec::new(), SerializationType::Raw);
        let start_clone = start.clone();
        let hdl_res = Context::register(start, &ctx.vals, Hdl::Val);
        get_val(&ctx, hdl_res).map(|f| assert_eq!(*f, start_clone))
    }

    #[test]
    fn round_trip_byte_array() -> Result<()> {
        let ctx = Context::default();
        let start = vec![1, 2, 3, 4, 5];
        let start_clone = start.clone();
        let hdl_res = Context::register(start, &ctx.byte_arrays, Hdl::ByteArray);
        get_byte_array_mut(&ctx, hdl_res).map(|b| assert_eq!(**b, start_clone))
    }

    #[test]
    fn remove_handle() -> Result<()> {
        let mut ctx = Context::default();
        let hdl = Context::register("hello".to_string(), &ctx.strings, Hdl::String);
        ctx.remove(hdl, |h| matches!(h, Hdl::String(_)));
        assert!(get_string(&ctx, hdl).is_err());
        Ok(())
    }
}
