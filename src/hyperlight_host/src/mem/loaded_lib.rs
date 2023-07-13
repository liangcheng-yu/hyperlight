use super::ptr::RawPtr;
use anyhow::{bail, Result};
use std::ffi::{c_char, CString};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use windows::core::PCSTR;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{FreeLibrary, LoadLibraryA};

static IS_RUNNING_FROM_GUEST_BINARY: AtomicBool = AtomicBool::new(false);

// Loadedlib needs to send so that it can be used in a Sandbox that can be passed between threads
// *mut c_char is not send, so that means LoadedLib cannot be Send.
// to work around this  we could try and wrap *mut c_char in a Mutex but in order for Mutex to be send
// *mut c_char would need to be sync which it is not.
// Additionally *muc c_char is impl !Send so we cannot unsafe impl Send for *mut c_char
// Therefore we need to wrap *mut c_char in a struct that is impl Send
// We also need to make this type Sync as it is wrapped in an Arc and Arc (just like Mutex) requires Sync in order to impl Send
// Marking this type Sync is safe as it is intended to only ever used from a single thread.

struct PtrCCharMut(*mut c_char);

unsafe impl Send for PtrCCharMut {}
unsafe impl Sync for PtrCCharMut {}

/// A wrapper around a binary loaded with the Windows
/// [`LoadLibraryA`](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/LibraryLoader/fn.LoadLibraryA.html)
/// function.
///
/// This struct ensures that globally, only one binary can be loaded at
/// at one time. It is concurrency safe and the `Drop` implementation
/// automatically unloads the binary with
/// [`FreeLibrary`](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/LibraryLoader/fn.FreeLibrary.html).
///
/// Use the `TryFrom` implementation to create a new instance.
#[derive(Clone)]
pub(crate) struct LoadedLib {
    data: Arc<(HMODULE, PtrCCharMut)>,
}

impl LoadedLib {
    pub(crate) fn base_addr(&self) -> Result<RawPtr> {
        let h_inst = self.data.0;
        let h_inst_u64: u64 = h_inst.0.try_into()?;
        Ok(RawPtr::from(h_inst_u64))
    }
}

/// frees `h_inst` using `FreeLibrary`, then frees `file_name_c_str` using
/// the standard `CString` drop functionality, in that order
unsafe fn free_and_drop(h_inst: HMODULE, file_name_c_str: *mut c_char) {
    FreeLibrary(h_inst);
    drop(CString::from_raw(file_name_c_str));
}

impl Drop for LoadedLib {
    fn drop(&mut self) {
        // if the ref count is greater than 1, this particular LoadedLib
        // has been cloned, so we don't want to free stuff yet
        if Arc::strong_count(&self.data) > 1 {
            return;
        }
        // the library, referenced by self.h_instance, owns
        // self.file_name. make sure they're freed in reverse order
        unsafe {
            free_and_drop(self.data.0, self.data.1 .0);
        }
        if !set_guest_binary_boolean(false) {
            // should never get here, in place just to catch bugs
            panic!("LoadedLib: could not set global guest binary boolean to false")
        }
    }
}

impl TryFrom<&str> for LoadedLib {
    type Error = anyhow::Error;
    fn try_from(file_name: &str) -> Result<Self> {
        let cstr = CString::new(file_name)?.into_raw();
        let file_name_pc_str = PCSTR::from_raw(cstr as *const u8);
        let h_instance = unsafe { LoadLibraryA(file_name_pc_str) }?;

        // ensure we set the atomic bool to true here _before_ creating
        // the actual instance, because the instance's drop will always
        // set the boolean to false.
        if !set_guest_binary_boolean(true) {
            unsafe {
                // safety: we just created h_instance and c_str
                free_and_drop(h_instance, cstr);
            }
            bail!("LoadedLib: could not set global guest binary boolean to true");
        }

        Ok(Self {
            data: Arc::new((h_instance, PtrCCharMut(cstr))),
        })
    }
}

/// do the following operation atomically on the internal global boolean that
/// indicates whether we're running directly from the guest binary:
///
/// - if it was set to `!val`, set it to `val` and return `true`
/// - otherwise, return `false`
fn set_guest_binary_boolean(val: bool) -> bool {
    // atomically set IS_RUNNING_FROM_GUEST_BINARY to true. if this returns
    // an Ok, the set operation succeeded
    IS_RUNNING_FROM_GUEST_BINARY
        .compare_exchange(!val, val, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::set_guest_binary_boolean;
    use super::LoadedLib;
    use crate::testing::{simple_guest_buf, simple_guest_path};
    #[cfg(not(RunningNextest))]
    use serial_test::serial;

    /// universal test for all LoadedLib-related functionality. It's necessary
    /// to put everything into a single test because LoadedLib relies on global
    /// state.
    #[test]
    #[cfg_attr(not(RunningNextest), serial)]
    fn test_universal() {
        // first, test the basic set_guest_binary_boolean
        {
            // should not be running, so mark running
            assert!(set_guest_binary_boolean(true));
            // should already be running, so marking running should return false
            assert!(!set_guest_binary_boolean(true));
            // now should be running, so mark not running
            assert!(set_guest_binary_boolean(false));
            // should not be running, so marking not running should return false
            assert!(!set_guest_binary_boolean(false));
        }
        // next, test basic load/unload functionality
        {
            // a test to just ensure we can load and unload (when dropped)
            // a library using LoadLibraryA and FreeLibrary, respectively
            let path = simple_guest_path().unwrap();
            let _ = LoadedLib::try_from(path.as_str()).unwrap();
        }
        // finally, actually test loading a library from a real compiled
        // binary
        {
            let lib_name = simple_guest_buf();
            let lib = LoadedLib::try_from(lib_name.to_str().unwrap()).unwrap();
            for _ in 0..9 {
                let l = lib.clone();
                assert_eq!(lib.base_addr().unwrap(), l.base_addr().unwrap());
            }
        }
    }
}
