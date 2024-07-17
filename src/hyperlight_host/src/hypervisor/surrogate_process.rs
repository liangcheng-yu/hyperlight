use tracing::{instrument, Span};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Memory::{VirtualFreeEx, MEM_RELEASE};

use super::surrogate_process_manager::get_surrogate_process_manager;
use crate::mem::shared_mem::PtrCVoidMut;

/// Contains details of a surrogate process to be used by a Sandbox for providing memory to a HyperV VM on Windows.
/// See surrogate_process_manager for details on why this is needed.
#[derive(Debug)]
pub(super) struct SurrogateProcess {
    /// The address of memory allocated in the surrogate process to be mapped to the VM.
    pub(crate) allocated_address: PtrCVoidMut,
    /// The handle to the surrogate process.
    pub(crate) process_handle: HANDLE,
}

impl SurrogateProcess {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn new(allocated_address: PtrCVoidMut, process_handle: HANDLE) -> Self {
        Self {
            allocated_address,
            process_handle,
        }
    }
}

impl Default for SurrogateProcess {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn default() -> Self {
        let allocated_address = PtrCVoidMut::from(std::ptr::null_mut());
        Self::new(allocated_address, Default::default())
    }
}

impl Drop for SurrogateProcess {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn drop(&mut self) {
        unsafe {
            if !VirtualFreeEx(
                self.process_handle,
                self.allocated_address.as_mut_ptr(),
                0,
                MEM_RELEASE,
            )
            .as_bool()
            {
                tracing::error!(
                    "Failed to free surrogate process resources (VirtualFreeEx failed)"
                );
            }
        }

        // we need to do this take so we can take ownership
        // of the SurrogateProcess being dropped. this is ok to
        // do because we are in the process of dropping ourselves
        // anyway.
        get_surrogate_process_manager()
            .unwrap()
            .return_surrogate_process(self.process_handle)
            .unwrap();
    }
}
