extern crate hyperlight_host;

use super::c_func::CFunc;
use super::hdl::Hdl;
use super::strings::register_string;
use super::{
    handle::{new_key, Handle, Key},
    sandbox_compat,
};
use crate::mem_access_handler::MemAccessHandlerWrapper;
use crate::outb_handler::OutBHandlerWrapper;
use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::FunctionCall, function_types::ReturnValue, guest_error::GuestError,
    guest_log_data::GuestLogData,
};
use hyperlight_host::error::HyperlightError;
#[cfg(target_os = "linux")]
use hyperlight_host::hypervisor::hyperv_linux::HypervLinuxDriver;
#[cfg(target_os = "linux")]
use hyperlight_host::hypervisor::kvm::KVMDriver;
use hyperlight_host::log_then_return;
use hyperlight_host::mem::mgr::SandboxMemoryManager;
use hyperlight_host::mem::shared_mem::SharedMemory;
use hyperlight_host::mem::shared_mem_snapshot::SharedMemorySnapshot;
use hyperlight_host::new_error;
use hyperlight_host::Result;
use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::sync::Once;
use tracing::info;
use tracing_subscriber;
use uuid::Uuid;

static INITTRACER: Once = Once::new();

/// The error message returned when a null reference check on a Context raw pointer fails in the C api.
pub(crate) const ERR_NULL_CONTEXT: &str = "NULL context was passed";

/// Return a null context error handle when Context is null.
#[macro_export]
macro_rules! validate_context {
    ($cob:ident) => {
        if $cob.is_null() {
            return Handle::new_null_context();
        }
    };
}

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
    /// The host's correlation Id for this context
    pub(crate) correlation_id: String,
    /// All `anyhow::Error`s stored in this context.
    pub(crate) errs: HashMap<Key, HyperlightError>,
    /// All booleans stored in this context
    pub(crate) booleans: HashMap<Key, bool>,
    /// All `Sandbox`es stored in this context
    pub(crate) sandboxes: HashMap<Key, sandbox_compat::Sandbox>,
    /// All `String`s stored in this context
    pub(crate) strings: HashMap<Key, String>,
    /// All `Vec<u8>`s stored in this context
    pub(crate) byte_arrays: HashMap<Key, Vec<u8>>,
    /// All the `SandboxMemoryManager`s stored in this context
    pub(crate) mem_mgrs: HashMap<Key, SandboxMemoryManager>,
    /// All the `SharedMemory`s stored in this context
    pub(crate) shared_mems: HashMap<Key, SharedMemory>,
    /// All the `SharedMemorySnapshot`s stored in this context
    pub(crate) shared_mem_snapshots: HashMap<Key, SharedMemorySnapshot>,
    /// All the `i64`s stored in this context
    pub(crate) int64s: HashMap<Key, i64>,
    /// All the `u64`s stored in this context
    pub(crate) uint64s: HashMap<Key, u64>,
    /// All the `i32`s stored in this context
    pub(crate) int32s: HashMap<Key, i32>,
    #[cfg(target_os = "linux")]
    /// The HyperV Linux VM drivers stored in this context
    pub(crate) hyperv_linux_drivers: HashMap<Key, HypervLinuxDriver>,
    #[cfg(target_os = "linux")]
    /// The KVM Linux VM drivers stored in this context
    pub(crate) kvm_drivers: HashMap<Key, KVMDriver>,
    /// The outb handler functions stored in this context
    pub(crate) outb_handler_funcs: HashMap<Key, OutBHandlerWrapper>,
    /// The memory access handler functions stored in this context
    pub(crate) mem_access_handler_funcs: HashMap<Key, MemAccessHandlerWrapper>,
    /// All the `GuestMemory`s stored in this context
    pub(crate) guest_errors: HashMap<Key, GuestError>,
    /// All the `FunctionCall`s stored in this context
    pub(crate) host_function_calls: HashMap<Key, FunctionCall>,
    /// All the `FunctionCallResult`s stored in this context
    pub(crate) function_call_results: HashMap<Key, ReturnValue>,
    /// All the `GuestLogData`s stored in this context
    pub(crate) guest_log_datas: HashMap<Key, GuestLogData>,
}

impl Context {
    /// Create a new key and register the given `obj` in the given
    /// collection `coll`.
    ///
    /// The given `FnOnce` called `make_handle` can be used to
    /// create a new `Handle` from the newly created key, and to
    /// verify that the given `obj` is of the correct type.
    pub(crate) fn register<T, HandleFn: FnOnce(Key) -> Hdl>(
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
    /// for `HyperlightError` types.
    pub(crate) fn register_err(&mut self, err: HyperlightError) -> Handle {
        Self::register(err, &mut self.errs, Hdl::Err)
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
    pub(crate) fn get<T, ChkFn: FnOnce(&Hdl) -> bool>(
        handle: Handle,
        coll: &HashMap<Key, T>,
        chk: ChkFn,
    ) -> Result<&T> {
        let hdl = Hdl::try_from(handle)?;
        if !chk(&hdl) {
            log_then_return!("invalid handle");
        }
        match coll.get(&handle.key()) {
            Some(obj) => Ok(obj),
            None => Err(new_error!(
                "object {} not found for key {}",
                hdl,
                handle.key()
            )),
        }
    }

    /// Similar to `get`, except returns a `WriteResult` rather than
    /// a `ReadResult`, making this function suitable for mutating
    /// `coll` in a thread-safe manner.
    pub(crate) fn get_mut<T, ChkFn: FnOnce(&Hdl) -> bool>(
        handle: Handle,
        coll: &mut HashMap<Key, T>,
        chk: ChkFn,
    ) -> Result<&mut T> {
        let hdl = Hdl::try_from(handle)?;
        if !chk(&hdl) {
            log_then_return!("invalid handle");
        }
        match coll.get_mut(&handle.key()) {
            Some(obj) => Ok(obj),
            None => {
                log_then_return!("object {} not found for key {}", hdl, handle.key());
            }
        }
    }

    /// Convert the given `Handle` parameter to a `Hdl` type (returning
    /// an `Err` if the conversion fails), then call `chk_fn` and
    /// immediately return an `Err` if it returns `false`, and finally
    /// remove that `Hdl`'s key from the collection that corresponds to
    /// it, returning `true` if an element was removed and `false`
    /// otherwise.
    pub(crate) fn remove<ChkFn>(&mut self, handle: Handle, chk_fn: ChkFn) -> bool
    where
        ChkFn: FnOnce(&Hdl) -> bool,
    {
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

/// Create a new `Context`.
///
/// # Safety
///
/// You must only call this function:
///
/// - With an optional correlation_id which should be a UTF-8 encoded string that is freed by the caller, if no correlation_id is provided, a new UUID will be generated
///
#[no_mangle]
pub unsafe extern "C" fn context_new(correlation_id: *const c_char) -> *mut Context {
    INITTRACER.call_once(|| {
        // TODO: Replace the subscriber with custom Hyperlight subscriber
        // TODO: Allow the host to set the tracing level
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::ERROR)
            .init();
    });
    let correlation_id = if correlation_id.is_null() {
        info!("No correlation id or function provided by host, generating one");
        Uuid::new_v4().to_string()
    } else {
        let cid = unsafe { CStr::from_ptr(correlation_id) };
        cid.to_string_lossy().into_owned()
    };
    let context = Context {
        correlation_id,
        ..Default::default()
    };
    Box::into_raw(Box::new(context))
}

/// Get the correlation_id associated with the Context.
///
/// # Safety
///
/// You must only call this function:
///
/// - With `Context`s created by `context_new`
/// - Before calling `context_free`
///
#[no_mangle]
pub unsafe extern "C" fn get_correlation_id(ctx: *mut Context) -> Handle {
    CFunc::new("get_correlation_id", ctx)
        .and_then_mut(|ctx, _| {
            let correlation_id = ctx.correlation_id.clone();
            Ok(register_string(&mut *ctx, correlation_id))
        })
        .ok_or_err_hdl()
}

/// Update the correlation_id associated with the Context.
///
/// # Safety
///
/// You must only call this function:
///
/// - With `Context`s created by `context_new`
/// - Before calling `context_free`
/// - With a new correlation_id which should be a UTF-8 encoded string that is freed by the caller if no correlation_id is provided, a new UUID will be generated
///

#[no_mangle]
pub unsafe extern "C" fn set_correlation_id(
    ctx: *mut Context,
    correlation_id: *const c_char,
) -> Handle {
    CFunc::new("set_correlation_id", ctx)
        .and_then_mut(|ctx, _| {
            let correlation_id = if correlation_id.is_null() {
                info!("No correlation id or function provided by host, generating one");
                Uuid::new_v4().to_string()
            } else {
                let cid = unsafe { CStr::from_ptr(correlation_id) };
                cid.to_string_lossy().into_owned()
            };
            ctx.correlation_id = correlation_id;
            Ok(Handle::new_empty())
        })
        .ok_or_err_hdl()
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
pub unsafe extern "C" fn context_free(ctx: *mut Context) -> Handle {
    validate_context!(ctx);
    drop(Box::from_raw(ctx));
    Handle::new_empty()
}

/// Panic when the Context is null.
#[macro_export]
macro_rules! validate_context_or_panic {
    ($cob:ident) => {
        if $cob.is_null() {
            // using the fully-qualified name for ERR_NULL_CONTEXT
            // here to avoid forcing macro users to import it
            // themselves.
            panic!("{}", $crate::context::ERR_NULL_CONTEXT);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::Context;
    use super::Handle;
    use crate::byte_array::get_byte_array;
    use crate::handle::NULL_CONTEXT_KEY;
    use crate::hdl::Hdl;
    use crate::strings::get_string;
    use crate::{validate_context, validate_context_or_panic};
    use hyperlight_host::Result;

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
