use std::fmt::Debug;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use tracing::{instrument, Span};

use crate::error::HyperlightError::NoHypervisorFound;
use crate::hypervisor::handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper};
#[cfg(mshv)]
use crate::hypervisor::hyperv_linux;
#[cfg(kvm)]
use crate::hypervisor::kvm;
use crate::hypervisor::Hypervisor;
use crate::mem::layout::SandboxMemoryLayout;
use crate::mem::mgr::SandboxMemoryManager;
use crate::mem::ptr::{GuestPtr, RawPtr};
use crate::mem::ptr_offset::Offset;
use crate::HyperlightError::LockAttemptFailed;
use crate::{log_then_return, Result, UninitializedSandbox};

static AVAILABLE_HYPERVISOR: OnceLock<Option<HypervisorType>> = OnceLock::new();

pub fn get_available_hypervisor() -> &'static Option<HypervisorType> {
    AVAILABLE_HYPERVISOR.get_or_init(|| {
        cfg_if::cfg_if! {
            if #[cfg(all(kvm, mshv))] {
                // If both features are enabled, we need to determine hypervisor at runtime.
                // Currently /dev/kvm and /dev/mshv cannot exist on the same machine, so the first one
                // that works is guaranteed to be correct.
                if hyperv_linux::is_hypervisor_present() {
                    Some(HypervisorType::Mshv)
                } else if kvm::is_hypervisor_present() {
                    Some(HypervisorType::Kvm)
                } else {
                    None
                }
            } else if #[cfg(kvm)] {
                if kvm::is_hypervisor_present() {
                    Some(HypervisorType::Kvm)
                } else {
                    None
                }
            } else if #[cfg(mshv)] {
                if hyperv_linux::is_hypervisor_present() {
                    Some(HypervisorType::Mshv)
                } else {
                    None
                }
            } else if #[cfg(target_os = "windows")] {
                use crate::sandbox::windows_hypervisor_platform;

                if windows_hypervisor_platform::is_hypervisor_present() {
                    Some(HypervisorType::Whp)
                } else {
                    None
                }
            } else {
                None
            }
        }
    })
}

/// The hypervisor types available for the current platform
#[derive(PartialEq, Eq, Debug)]
pub(crate) enum HypervisorType {
    #[cfg(kvm)]
    Kvm,

    #[cfg(mshv)]
    Mshv,

    #[cfg(target_os = "windows")]
    Whp,
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

    /// Try to get the lock for `max_execution_time` duration
    pub(crate) fn try_get_hypervisor_lock(&self) -> Result<MutexGuard<Box<dyn Hypervisor>>> {
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
                                "try_get_hypervisor_lock failed".to_string()
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
        let mut regions = mgr.layout.get_memory_regions(&mgr.shared_mem)?;
        let rsp_ptr = {
            let rsp_u64 = mgr.set_up_hypervisor_partition(mem_size, &mut regions)?;
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

        match *get_available_hypervisor() {
            #[cfg(mshv)]
            Some(HypervisorType::Mshv) => {
                use crate::hypervisor::hyperv_linux::HypervLinuxDriver;

                let hv = HypervLinuxDriver::new(regions, entrypoint_ptr, rsp_ptr, pml4_ptr)?;
                Ok(Box::new(hv))
            }

            #[cfg(kvm)]
            Some(HypervisorType::Kvm) => {
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
            Some(HypervisorType::Whp) => {
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

            _ => {
                log_then_return!(NoHypervisorFound());
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
