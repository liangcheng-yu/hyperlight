use super::handle::{new_key, Handle, Key};
use super::hdl::Hdl;
use crate::func::args::Val;
use crate::func::def::HostFunc;
use crate::mem::pe::PEInfo;
use crate::sandbox::Sandbox;
use anyhow::{anyhow, bail, Error, Result};
use chashmap::{CHashMap, ReadGuard, WriteGuard};

#[derive(Default)]
pub struct Context {
    errs: CHashMap<Key, Error>,
    sandboxes: CHashMap<Key, Sandbox>,
    vals: CHashMap<Key, Val>,
    host_funcs: CHashMap<Key, HostFunc>,
    strings: CHashMap<Key, String>,
    byte_arrays: CHashMap<Key, Vec<u8>>,
    pe_infos: CHashMap<Key, PEInfo>,
}

pub type ReadResult<'a, T> = Result<ReadGuard<'a, Key, T>>;
pub type WriteResult<'a, T> = Result<WriteGuard<'a, Key, T>>;

impl Context {
    fn register<T, HandleFn: FnOnce(Key, &T) -> Handle>(
        obj: T,
        coll: &CHashMap<Key, T>,
        make_handle: HandleFn,
    ) -> Handle {
        let key = new_key();
        let handle = make_handle(key, &obj);
        coll.insert(handle.key(), obj);
        handle
    }

    fn get<T, CheckFn: FnOnce(Handle) -> Result<()>>(
        handle: Handle,
        coll: &CHashMap<Key, T>,
        check_fn: CheckFn,
    ) -> ReadResult<T> {
        check_fn(handle)?;
        match coll.get(&handle.key()) {
            Some(obj) => Ok(obj),
            None => bail!("object not found for key {}", handle.key()),
        }
    }

    fn get_mut<T, CheckFn: FnOnce(Handle) -> Result<()>>(
        handle: Handle,
        coll: &CHashMap<Key, T>,
        check_fn: CheckFn,
    ) -> WriteResult<T> {
        check_fn(handle)?;
        match coll.get_mut(&handle.key()) {
            Some(obj) => Ok(obj),
            None => bail!("object not found for key {}", handle.key()),
        }
    }

    pub fn register_err(&self, err: Error) -> Handle {
        Self::register(err, &self.errs, |key, _| Handle::from(Hdl::Err(key)))
    }

    pub fn get_err(&self, handle: Handle) -> ReadResult<Error> {
        Self::get(handle, &self.errs, |handle| match Hdl::try_from(handle) {
            Ok(Hdl::Err(_)) => Ok(()),
            _ => bail!("handle is not an error handle"),
        })
    }

    pub fn register_sandbox(&self, sbox: Sandbox) -> Handle {
        Self::register(sbox, &self.sandboxes, |key, _| {
            Handle::from(Hdl::Sandbox(key))
        })
    }

    pub fn get_sandbox(&self, handle: Handle) -> ReadResult<Sandbox> {
        Self::get(handle, &self.sandboxes, |handle| {
            match Hdl::try_from(handle) {
                Ok(Hdl::Sandbox(_)) => Ok(()),
                _ => bail!("handle is not a sandbox handle"),
            }
        })
    }

    pub fn get_sandbox_mut(&self, handle: Handle) -> WriteResult<Sandbox> {
        Self::get_mut(handle, &self.sandboxes, |handle| {
            match Hdl::try_from(handle) {
                Ok(Hdl::Sandbox(_)) => Ok(()),
                _ => bail!("handle is not a sandbox handle"),
            }
        })
    }

    pub fn register_val(&self, val: Val) -> Handle {
        Self::register(val, &self.vals, |key, _| Handle::from(Hdl::Val(key)))
    }

    pub fn get_val(&self, handle: Handle) -> ReadResult<Val> {
        Self::get(handle, &self.vals, |handle| match Hdl::try_from(handle) {
            Ok(Hdl::Val(_)) => Ok(()),
            _ => bail!("handle is not a val handle"),
        })
    }

    pub fn register_host_func(&self, func: HostFunc) -> Handle {
        Self::register(func, &self.host_funcs, |key, _| {
            Handle::from(Hdl::HostFunc(key))
        })
    }
    pub fn get_host_func(&self, handle: Handle) -> ReadResult<HostFunc> {
        Self::get(handle, &self.host_funcs, |handle| {
            match Hdl::try_from(handle) {
                Ok(Hdl::HostFunc(_)) => Ok(()),
                _ => bail!("handle is not a host func handle"),
            }
        })
    }

    pub fn register_string(&self, string: String) -> Handle {
        Self::register(string, &self.strings, |key, _| {
            Handle::from(Hdl::String(key))
        })
    }
    pub fn get_string(&self, handle: Handle) -> ReadResult<String> {
        Self::get(handle, &self.strings, |handle| {
            match Hdl::try_from(handle) {
                Ok(Hdl::String(_)) => Ok(()),
                _ => bail!("handle is not a string handle"),
            }
        })
    }

    pub fn register_byte_array(&self, bytes: Vec<u8>) -> Handle {
        Self::register(bytes, &self.byte_arrays, |key, _| {
            Handle::from(Hdl::ByteArray(key))
        })
    }

    pub fn get_byte_array_mut(&self, handle: Handle) -> WriteResult<Vec<u8>> {
        Self::get_mut(handle, &self.byte_arrays, |handle| {
            match Hdl::try_from(handle) {
                Ok(Hdl::ByteArray(_)) => Ok(()),
                _ => bail!("handle is not a byte array handle"),
            }
        })
    }

    pub fn get_byte_array(&self, handle: Handle) -> ReadResult<Vec<u8>> {
        Self::get(handle, &self.byte_arrays, |handle| {
            match Hdl::try_from(handle) {
                Ok(Hdl::ByteArray(_)) => Ok(()),
                _ => bail!("handle is not a byte array handle"),
            }
        })
    }

    pub fn remove_byte_array(&self, handle: Handle) -> Option<Vec<u8>> {
        self.byte_arrays.remove(&handle.key())
    }

    pub fn get_pe_info(&self, handle: Handle) -> ReadResult<PEInfo> {
        match Hdl::try_from(handle) {
            Ok(Hdl::PEInfo(_)) => {}
            _ => bail!("handle is not a PE handle"),
        }
        let res = self.pe_infos.get(&handle.key());
        res.ok_or_else(|| anyhow!("PE not found for handle {}", handle.key()))
    }

    pub fn register_pe_info(&mut self, pe: PEInfo) -> Handle {
        Self::register(pe, &self.pe_infos, |key, _| Handle::from(Hdl::PEInfo(key)))
    }

    pub fn remove_handle(&mut self, handle: Handle) -> bool {
        match Hdl::try_from(handle) {
            Ok(Hdl::Err(key)) => self.errs.remove(&key).is_some(),
            Ok(Hdl::Sandbox(key)) => self.sandboxes.remove(&key).is_some(),
            Ok(Hdl::Empty()) => true,
            Ok(Hdl::Val(key)) => self.vals.remove(&key).is_some(),
            Ok(Hdl::HostFunc(key)) => self.host_funcs.remove(&key).is_some(),
            Ok(Hdl::String(key)) => self.strings.remove(&key).is_some(),
            Ok(Hdl::ByteArray(key)) => self.byte_arrays.remove(&key).is_some(),
            Ok(Hdl::PEInfo(key)) => self.pe_infos.remove(&key).is_some(),
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
    use crate::func::args::Val;
    use crate::func::SerializationType;
    use anyhow::Result;

    #[test]
    fn round_trip_string() -> Result<()> {
        let ctx = Context::default();
        let start = "hello".to_string();
        let hdl_res = ctx.register_string(start.clone());
        ctx.get_string(hdl_res).map(|f| assert_eq!(*f, start))
    }

    #[test]
    fn round_trip_val() -> Result<()> {
        let ctx = Context::default();
        let start = Val::new(Vec::new(), SerializationType::Raw);
        let hdl_res = ctx.register_val(start.clone());
        ctx.get_val(hdl_res).map(|f| assert_eq!(*f, start))
    }

    #[test]
    fn round_trip_byte_array() -> Result<()> {
        let ctx = Context::default();
        let start = vec![1, 2, 3, 4, 5];
        let hdl_res = ctx.register_byte_array(start.clone());
        ctx.get_byte_array_mut(hdl_res)
            .map(|b| assert_eq!(**b, start))
    }

    #[test]
    fn remove_handle() -> Result<()> {
        let mut ctx = Context::default();
        let hdl = ctx.register_string("hello".to_string());
        ctx.remove_handle(hdl);
        assert!(ctx.get_string(hdl).is_err());
        Ok(())
    }
}
