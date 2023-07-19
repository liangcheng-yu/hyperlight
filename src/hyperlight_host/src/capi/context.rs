use super::hdl::Hdl;
use super::{
    handle::{new_key, Handle, Key},
    sandbox_compat,
};
use crate::capi::outb_handler::OutBHandlerWrapper;
use crate::func::function_call::FunctionCall;
use crate::func::guest::error::GuestError;
use crate::func::types::ReturnValue;
#[cfg(target_os = "linux")]
use crate::hypervisor::hyperv_linux::HypervLinuxDriver;
#[cfg(target_os = "linux")]
use crate::hypervisor::kvm::KVMDriver;
use crate::mem::layout::SandboxMemoryLayout;
use crate::mem::mgr::SandboxMemoryManager;
use crate::mem::shared_mem::SharedMemory;
use crate::mem::shared_mem_snapshot::SharedMemorySnapshot;
use crate::{
    capi::mem_access_handler::MemAccessHandlerWrapper, func::guest::log_data::GuestLogData,
};
use anyhow::{bail, Error, Result};
use std::collections::HashMap;

/// Context is a memory storage mechanism used in the Hyperlight C API
/// functions.
///
/// It is intended to be referred to by `Handle`s, which are passed
/// between C code and Rust implementation herein as the rough equivalent
/// of pointers.
///
/// Using this `Handle` and `Context` scheme to refer to allocated
/// memory provides a somewhat safer, though less efficient, way
/// to refer to memory on the heap than "raw" C pointers do.
///
/// # Safety
///
/// - Wherever a `Context` pointer is expected in a C function, you should
/// always pass a pointer returned to you by the `context_new` function,
/// that you have not modified in any way or passed to `context_free`.
/// - Functions that return a `Handle` often write new data into the
/// `Context`
/// - `Context` is not thread-safe. Do not share one between threads
#[derive(Default)]
pub struct Context {
    /// All `anyhow::Error`s stored in this context.
    pub errs: HashMap<Key, Error>,
    /// All booleans stored in this context
    pub booleans: HashMap<Key, bool>,
    /// All `Sandbox`es stored in this context
    pub sandboxes: HashMap<Key, sandbox_compat::Sandbox>,
    /// All `String`s stored in this context
    pub strings: HashMap<Key, String>,
    /// All `Vec<u8>`s stored in this context
    pub byte_arrays: HashMap<Key, Vec<u8>>,
    /// All `SandboxMemoryLayout`s stored in this context
    pub mem_layouts: HashMap<Key, SandboxMemoryLayout>,
    /// All the `SandboxMemoryManager`s stored in this context
    pub mem_mgrs: HashMap<Key, SandboxMemoryManager>,
    /// All the `SharedMemory`s stored in this context
    pub shared_mems: HashMap<Key, SharedMemory>,
    /// All the `SharedMemorySnapshot`s stored in this context
    pub shared_mem_snapshots: HashMap<Key, SharedMemorySnapshot>,
    /// All the `i64`s stored in this context
    pub int64s: HashMap<Key, i64>,
    /// All the `u64`s stored in this context
    pub uint64s: HashMap<Key, u64>,
    /// All the `i32`s stored in this context
    pub int32s: HashMap<Key, i32>,
    #[cfg(target_os = "linux")]
    /// The HyperV Linux VM drivers stored in this context
    pub hyperv_linux_drivers: HashMap<Key, HypervLinuxDriver>,
    #[cfg(target_os = "linux")]
    /// The KVM Linux VM drivers stored in this context
    pub kvm_drivers: HashMap<Key, KVMDriver>,
    /// The outb handler functions stored in this context
    pub outb_handler_funcs: HashMap<Key, OutBHandlerWrapper>,
    /// The memory access handler functions stored in this context
    pub mem_access_handler_funcs: HashMap<Key, MemAccessHandlerWrapper>,
    /// All the `GuestMemory`s stored in this context
    pub guest_errors: HashMap<Key, GuestError>,
    /// All the `FunctionCall`s stored in this context
    pub host_function_calls: HashMap<Key, FunctionCall>,
    /// All the `FunctionCallResult`s stored in this context
    pub function_call_results: HashMap<Key, ReturnValue>,
    /// All the `GuestLogData`s stored in this context
    pub guest_log_datas: HashMap<Key, GuestLogData>,
}

impl Context {
    /// Create a new key and register the given `obj` in the given
    /// collection `coll`.
    ///
    /// The given `FnOnce` called `make_handle` can be used to
    /// create a new `Handle` from the newly created key, and to
    /// verify that the given `obj` is of the correct type.
    pub fn register<T, HandleFn: FnOnce(Key) -> Hdl>(
        obj: T,
        coll: &mut HashMap<Key, T>,
        make_handle: HandleFn,
    ) -> Handle {
        let key = new_key();
        let handle = Handle::from(make_handle(key));
        coll.insert(handle.key(), obj);
        handle
    }

    /// A convenience function for `register`, typed specifically
    /// for `Error` types.
    pub fn register_err(&mut self, err: Error) -> Handle {
        Self::register(err, &mut self.errs, Hdl::Err)
    }

    /// Convenience method for:
    /// ```
    /// self.register_err(anyhow::Error::msg(err_msg))
    /// ```
    pub fn register_err_msg(&mut self, err_msg: &str) -> Handle {
        self.register_err(anyhow::Error::msg(err_msg.to_string()))
    }

    /// Get a type `T` from the given collection `coll` using
    /// `handle.key()` as the index to `coll`.
    ///
    /// The `chk` function will be called with the `Hdl` created
    /// from the given `handle`, and if it returns `false`, an
    /// `Err` will be returned.
    ///
    /// This function is only suitable for immutable operations on
    /// `coll`. If you intend to mutate `coll`, use `get_mut`.
    pub fn get<T, ChkFn: FnOnce(&Hdl) -> bool>(
        handle: Handle,
        coll: &HashMap<Key, T>,
        chk: ChkFn,
    ) -> Result<&T> {
        let hdl = Hdl::try_from(handle)?;
        if !chk(&hdl) {
            bail!("invalid handle")
        }
        match coll.get(&handle.key()) {
            Some(obj) => Ok(obj),
            None => bail!("object {} not found for key {}", hdl, handle.key()),
        }
    }

    /// Similar to `get`, except returns a `WriteResult` rather than
    /// a `ReadResult`, making this function suitable for mutating
    /// `coll` in a thread-safe manner.
    pub fn get_mut<T, ChkFn: FnOnce(&Hdl) -> bool>(
        handle: Handle,
        coll: &mut HashMap<Key, T>,
        chk: ChkFn,
    ) -> Result<&mut T> {
        let hdl = Hdl::try_from(handle)?;
        if !chk(&hdl) {
            bail!("invalid handle")
        }
        match coll.get_mut(&handle.key()) {
            Some(obj) => Ok(obj),
            None => bail!("object {} not found for key {}", hdl, handle.key()),
        }
    }

    /// Convert the given `Handle` parameter to a `Hdl` type (returning
    /// an `Err` if the conversion fails), then call `chk_fn` and
    /// immediately return an `Err` if it returns `false`, and finally
    /// remove that `Hdl`'s key from the collection that corresponds to
    /// it, returning `true` if an element was removed and `false`
    /// otherwise.
    pub fn remove<ChkFn: FnOnce(&Hdl) -> bool>(&mut self, handle: Handle, chk_fn: ChkFn) -> bool {
        match Hdl::try_from(handle) {
            Ok(hdl) => {
                if !chk_fn(&hdl) {
                    return false;
                }
                match hdl {
                    Hdl::Err(key) => self.errs.remove(&key).is_some(),
                    Hdl::Boolean(key) => self.booleans.remove(&key).is_some(),
                    Hdl::Sandbox(key) => self.sandboxes.remove(&key).is_some(),
                    Hdl::Empty() => true,
                    Hdl::Invalid() => true,
                    Hdl::NullContext() => true,
                    Hdl::String(key) => self.strings.remove(&key).is_some(),
                    Hdl::ByteArray(key) => self.byte_arrays.remove(&key).is_some(),
                    Hdl::MemLayout(key) => self.mem_layouts.remove(&key).is_some(),
                    Hdl::MemMgr(key) => self.mem_mgrs.remove(&key).is_some(),
                    Hdl::SharedMemory(key) => self.shared_mems.remove(&key).is_some(),
                    Hdl::SharedMemorySnapshot(key) => {
                        self.shared_mem_snapshots.remove(&key).is_some()
                    }
                    Hdl::Int64(key) => self.int64s.remove(&key).is_some(),
                    Hdl::UInt64(key) => self.uint64s.remove(&key).is_some(),
                    Hdl::Int32(key) => self.int32s.remove(&key).is_some(),
                    #[cfg(target_os = "linux")]
                    Hdl::HypervLinuxDriver(key) => self.hyperv_linux_drivers.remove(&key).is_some(),
                    #[cfg(target_os = "linux")]
                    Hdl::KVMDriver(key) => self.kvm_drivers.remove(&key).is_some(),
                    Hdl::OutbHandlerFunc(key) => self.outb_handler_funcs.remove(&key).is_some(),
                    Hdl::MemAccessHandlerFunc(key) => {
                        self.mem_access_handler_funcs.remove(&key).is_some()
                    }
                    Hdl::GuestError(key) => self.guest_errors.remove(&key).is_some(),
                    Hdl::HostFunctionCall(key) => self.host_function_calls.remove(&key).is_some(),
                    Hdl::ReturnValue(key) => self.function_call_results.remove(&key).is_some(),
                    Hdl::GuestLogData(key) => self.guest_log_datas.remove(&key).is_some(),
                }
            }
            Err(_) => false,
        }
    }
}

/// Create a new context for use in the C API.
#[no_mangle]
pub extern "C" fn context_new() -> *mut Context {
    Box::into_raw(Box::default())
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
    drop(Box::from_raw(ctx))
}

/// The error message returned when a null reference check on a Context raw pointer fails in the C api.
pub const ERR_NULL_CONTEXT: &str = "NULL context was passed";

/// Return a null context error handle when Context is null.
#[macro_export]
macro_rules! validate_context {
    ($cob:ident) => {
        if $cob.is_null() {
            return Handle::new_null_context();
        }
    };
}

/// Panic when the Context is null.
#[macro_export]
macro_rules! validate_context_or_panic {
    ($cob:ident) => {
        if $cob.is_null() {
            // using the fully-qualified name for ERR_NULL_CONTEXT
            // here to avoid forcing macro users to import it
            // themselves.
            panic!("{}", $crate::capi::context::ERR_NULL_CONTEXT);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::Context;
    use super::Handle;
    use crate::capi::byte_array::get_byte_array;
    use crate::capi::handle::NULL_CONTEXT_KEY;
    use crate::capi::hdl::Hdl;
    use crate::capi::strings::get_string;
    use crate::{validate_context, validate_context_or_panic};
    use anyhow::Result;

    #[test]
    fn round_trip_string() -> Result<()> {
        let mut ctx = Context::default();
        let start = "hello".to_string();
        let hdl_res = Context::register(start, &mut ctx.strings, Hdl::String);
        Context::get(hdl_res, &ctx.strings, |s| matches!(s, Hdl::String(_)))?;
        Ok(())
    }

    #[test]
    fn round_trip_byte_array() -> Result<()> {
        let mut ctx = Context::default();
        let start = vec![1, 2, 3, 4, 5];
        let start_clone = start.clone();
        let hdl_res = Context::register(start, &mut ctx.byte_arrays, Hdl::ByteArray);
        get_byte_array(&ctx, hdl_res).map(|b| assert_eq!(**b, start_clone))
    }

    #[test]
    fn remove_handle() -> Result<()> {
        let mut ctx = Context::default();
        let hdl = Context::register("hello".to_string(), &mut ctx.strings, Hdl::String);
        ctx.remove(hdl, |h| matches!(h, Hdl::String(_)));
        assert!(get_string(&ctx, hdl).is_err());
        Ok(())
    }

    #[test]
    fn test_validate_context() -> Result<()> {
        let hdl = validate_context_test_helper();
        if hdl.key() != NULL_CONTEXT_KEY {
            assert_eq!(NULL_CONTEXT_KEY, hdl.key());
        }

        Ok(())
    }

    #[test]
    #[should_panic(expected = "NULL context was passed")]
    fn test_validate_context_or_panic() {
        let ctx: *mut Context = core::ptr::null_mut();
        validate_context_or_panic!(ctx);
    }

    fn validate_context_test_helper() -> Handle {
        let ctx: *mut Context = core::ptr::null_mut();
        validate_context!(ctx);

        Handle::new_empty()
    }
}
