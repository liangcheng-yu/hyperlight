use crate::error::HyperlightError::NoHypervisorFound;
use crate::HyperlightError::LockAttemptFailed;
use crate::{
    hypervisor::handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper},
    hypervisor::Hypervisor,
    mem::{
        layout::SandboxMemoryLayout,
        mgr::SandboxMemoryManager,
        ptr::{GuestPtr, RawPtr},
        ptr_offset::Offset,
    },
    UninitializedSandbox,
};
use crate::{log_then_return, Result};
use lazy_static::lazy_static;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use std::{sync::MutexGuard, time::Duration};
use tracing::{instrument, Span};

lazy_static! {
    /// The hypervisor available for the current platform, and is
    /// lazily initialized the first time it is accessed
    static ref AVAILABLE_HYPERVISOR: HypervisorType = {
        #[cfg(target_os = "linux")]
        {
            if crate::hypervisor::hyperv_linux::is_hypervisor_present() {
                HypervisorType::HyperVLinux
            } else if crate::hypervisor::kvm::is_hypervisor_present() {
                HypervisorType::Kvm
            } else {
                HypervisorType::None
            }
        }
        #[cfg(target_os = "windows")]
        {
            if crate::hypervisor::windows_hypervisor_platform::is_hypervisor_present() {
                HypervisorType::HyperV
            } else {
                HypervisorType::None
            }
        }
    };

}

/// The hypervisor types available for the current platform
enum HypervisorType {
    None,

    #[cfg(target_os = "linux")]
    Kvm,

    #[cfg(target_os = "linux")]
    HyperVLinux,

    #[cfg(target_os = "windows")]
    HyperV,
}

/// A container with convenience methods attached for an
/// `Option<Box<dyn Hypervisor>>`
#[derive(Clone)]
pub(crate) struct HypervisorWrapper {
    hv_opt: Option<Arc<Mutex<Box<dyn Hypervisor>>>>,
    pub(crate) outb_hdl: OutBHandlerWrapper,
    pub(crate) mem_access_hdl: MemAccessHandlerWrapper,
    pub(crate) max_execution_time: Duration,
    #[cfg(target_os = "linux")]
    pub(crate) max_wait_for_cancellation: Duration,
}

impl HypervisorWrapper {
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    pub(crate) fn new(
        hv_opt_box: Option<Box<dyn Hypervisor>>,
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
        max_execution_time: Duration,
        #[cfg(target_os = "linux")] max_wait_for_cancellation: Duration,
    ) -> Self {
        Self {
            hv_opt: hv_opt_box.map(|hv| {
                let mutx = Mutex::from(hv);
                Arc::from(mutx)
            }),
            outb_hdl,
            mem_access_hdl,
            max_execution_time,
            #[cfg(target_os = "linux")]
            max_wait_for_cancellation,
        }
    }

    /// if an internal `Hypervisor` exists, lock it and return a `MutexGuard`
    /// containing it.
    ///
    /// This `MutexGuard` represents exclusive read/write ownership of
    /// the underlying `Hypervisor`, so if this method returns an `Ok`,
    /// the value inside that `Ok` can be written or read.
    ///
    /// When the returned `MutexGuard` goes out of scope, the underlying lock
    /// will be released and the read/write guarantees will no longer be
    /// valid (the compiler won't let you do any operations on it, though,
    /// so you don't have to worry much about this consequence).
    pub(crate) fn get_hypervisor_lock(&self) -> Result<MutexGuard<Box<dyn Hypervisor>>> {
        match self.hv_opt.as_ref() {
            None => {
                log_then_return!(NoHypervisorFound());
            }
            Some(h_arc_mut) => {
                let h_ref_mutex = Arc::as_ref(h_arc_mut);

                Ok(h_ref_mutex
                    .lock()
                    .map_err(|_| LockAttemptFailed("get_hypervisor_lock failed".to_string()))?)
            }
        }
    }

    pub(crate) fn try_get_hypervisor_lock(&self) -> Result<MutexGuard<Box<dyn Hypervisor>>> {
        match self.hv_opt.as_ref() {
            None => {
                log_then_return!(NoHypervisorFound());
            }
            Some(h_arc_mut) => {
                let h_ref_mutex = Arc::as_ref(h_arc_mut);

                Ok(h_ref_mutex
                    .try_lock()
                    .map_err(|_| LockAttemptFailed("get_hypervisor_lock failed".to_string()))?)
            }
        }
    }

    /// Try to get the lock for `max_execution_time` duration
    pub(crate) fn try_get_hypervisor_lock_for_max_execution_time(
        &self,
    ) -> Result<MutexGuard<Box<dyn Hypervisor>>> {
        let timeout = self.max_execution_time;
        let start = Instant::now();

        match self.hv_opt.as_ref() {
            None => {
                log_then_return!(NoHypervisorFound());
            }
            Some(h_arc_mut) => {
                let h_ref_mutex = Arc::as_ref(h_arc_mut);

                loop {
                    match h_ref_mutex.try_lock() {
                        Ok(guard) => return Ok(guard),
                        Err(_) if start.elapsed() >= timeout => {
                            log_then_return!(LockAttemptFailed(
                                "try_get_hypervisor_lock_for_max_execution_time failed".to_string()
                            ));
                        }
                        Err(_) => {
                            // Sleep for a short duration to avoid busy-waiting
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            }
        }
    }

    /// if an internal `Hypervisor` exists, return Arc<Mutex<Box<dyn Hypervisor>>>
    /// containing it.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub(crate) fn get_hypervisor_arc(&self) -> Result<Arc<Mutex<Box<dyn Hypervisor>>>> {
        match self.hv_opt.as_ref() {
            None => {
                log_then_return!(NoHypervisorFound());
            }
            Some(h_arc_mut) => Ok(h_arc_mut.clone()),
        }
    }
}

impl UninitializedSandbox {
    /// Set up the appropriate hypervisor for the platform
    ///
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub(super) fn set_up_hypervisor_partition(
        mgr: &mut SandboxMemoryManager,
    ) -> Result<Box<dyn Hypervisor>> {
        let mem_size = u64::try_from(mgr.shared_mem.mem_size())?;
        let regions = mgr.layout.get_memory_regions(&mgr.shared_mem);
        let rsp_ptr = {
            let rsp_u64 = mgr.set_up_hypervisor_partition(mem_size)?;
            let rsp_raw = RawPtr::from(rsp_u64);
            GuestPtr::try_from(rsp_raw)
        }?;
        let base_ptr = GuestPtr::try_from(Offset::from(0))?;
        let pml4_ptr = {
            let pml4_offset_u64 = u64::try_from(SandboxMemoryLayout::PML4_OFFSET)?;
            base_ptr + Offset::from(pml4_offset_u64)
        };
        let entrypoint_ptr = {
            let entrypoint_total_offset = mgr.load_addr.clone() + mgr.entrypoint_offset;
            GuestPtr::try_from(entrypoint_total_offset)
        }?;

        assert!(base_ptr == pml4_ptr);
        assert!(entrypoint_ptr > pml4_ptr);
        assert!(rsp_ptr > entrypoint_ptr);

        match *AVAILABLE_HYPERVISOR {
            HypervisorType::None => {
                log_then_return!(NoHypervisorFound());
            }

            #[cfg(target_os = "linux")]
            HypervisorType::HyperVLinux => {
                use crate::hypervisor::hyperv_linux::HypervLinuxDriver;

                let hv = HypervLinuxDriver::new(regions, entrypoint_ptr, rsp_ptr, pml4_ptr)?;
                Ok(Box::new(hv))
            }

            #[cfg(target_os = "linux")]
            HypervisorType::Kvm => {
                use crate::hypervisor::kvm::KVMDriver;

                let hv = KVMDriver::new(
                    regions,
                    pml4_ptr.absolute()?,
                    entrypoint_ptr.absolute()?,
                    rsp_ptr.absolute()?,
                )?;
                Ok(Box::new(hv))
            }

            #[cfg(target_os = "windows")]
            HypervisorType::HyperV => {
                use crate::hypervisor::hyperv_windows::HypervWindowsDriver;

                let hv = HypervWindowsDriver::new(
                    regions,
                    mgr.shared_mem.raw_mem_size(), // we use raw_* here because windows driver requires 64K aligned addresses,
                    mgr.shared_mem.raw_ptr(), // and instead convert it to base_addr where needed in the driver itself
                    pml4_ptr.absolute()?,
                    entrypoint_ptr.absolute()?,
                    rsp_ptr.absolute()?,
                )?;
                Ok(Box::new(hv))
            }
        }
    }
}

impl Debug for HypervisorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HypervisorWrapper")
            .field("has_hypervisor", &self.hv_opt.is_some())
            .finish()
    }
}
