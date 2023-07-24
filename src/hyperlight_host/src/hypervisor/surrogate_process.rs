use core::ffi::c_void;
use windows::Win32::Foundation::HANDLE;
/// Contains details of a surrogate process to be used by a Sandbox for providing memory to a HyperV VM on Windows.
/// See surrogate_process_manager for details on why this is needed.

pub(crate) struct SurrogateProcess {
    /// The address of memory allocated in the surrogate process to be mapped to the VM.
    pub(crate) allocated_address: *mut c_void,
    /// The handle to the surrogate process.
    pub(crate) process_handle: HANDLE,
}

impl Default for SurrogateProcess {
    fn default() -> SurrogateProcess {
        Self {
            allocated_address: std::ptr::null_mut::<c_void>(),
            process_handle: Default::default(),
        }
    }
}
