use std::{sync::MutexGuard, time::Duration};

use crate::{
    func::exports::get_os_page_size,
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
use anyhow::{anyhow, bail, Result};
use rand::Rng;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

/// A container with convenience methods attached for an
/// `Option<Box<dyn Hypervisor>>`
#[derive(Clone)]
pub struct HypervisorWrapper<'a> {
    hv_opt: Option<Arc<Mutex<Box<dyn Hypervisor>>>>,
    outb_hdl: OutBHandlerWrapper<'a>,
    mem_access_hdl: MemAccessHandlerWrapper<'a>,
    max_execution_time: Duration,
    max_wait_for_cancellation: Duration,
}
/// A trait for getting a `HypervisorWrapper` from a type
pub trait HypervisorWrapperMgr<'a> {
    /// Get a reference to a `HypervisorWrapper` stored inside `self`
    fn get_hypervisor_wrapper(&self) -> &HypervisorWrapper<'a>;
    /// Get a mutable reference to a `HypervisorWrapper` stored inside `self`.
    /// Return an `Err` if no mutable reference can be provided. Such an error
    /// will most likely be returned when the `HypervisorWrapper` is stored
    /// inside an `Rc` or `Arc`, and there is at least one other clone of it,
    /// making a mutable reference impossible to get.
    fn get_hypervisor_wrapper_mut(&mut self) -> &mut HypervisorWrapper<'a>;
}

impl<'a> HypervisorWrapper<'a> {
    pub(super) fn new(
        hv_opt_box: Option<Box<dyn Hypervisor>>,
        outb_hdl: OutBHandlerWrapper<'a>,
        mem_access_hdl: MemAccessHandlerWrapper<'a>,
        max_execution_time: Duration,
        max_wait_for_cancellation: Duration,
    ) -> Self {
        Self {
            hv_opt: hv_opt_box.map(|hv| {
                let mutx = Mutex::from(hv);
                Arc::from(mutx)
            }),
            outb_hdl,
            mem_access_hdl,
            max_execution_time,
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
    pub(crate) fn get_hypervisor(&self) -> Result<MutexGuard<Box<dyn Hypervisor>>> {
        match self.hv_opt.as_ref() {
            None => bail!("no hypervisor available for sandbox"),
            Some(h_arc_mut) => {
                let h_ref_mutex = Arc::as_ref(h_arc_mut);
                h_ref_mutex
                    .lock()
                    .map_err(|_| anyhow!("unable to lock hypervisor"))
            }
        }
    }

    pub(super) fn get_outb_hdl_wrapper(&self) -> OutBHandlerWrapper<'a> {
        self.outb_hdl.clone()
    }

    pub(super) fn initialise(&mut self, mem_mgr: &SandboxMemoryManager) -> Result<()> {
        let seed = {
            let mut rng = rand::thread_rng();
            rng.gen::<u64>()
        };
        let peb_addr = {
            let peb_u64 = u64::try_from(mem_mgr.layout.peb_address)?;
            RawPtr::from(peb_u64)
        };
        let page_size = u32::try_from(get_os_page_size())?;
        let outb_hdl = self.outb_hdl.clone();
        let mem_access_hdl = self.mem_access_hdl.clone();
        let max_execution_time = self.max_execution_time;
        let max_wait_for_cancellation = self.max_wait_for_cancellation;
        let mut hv = self.get_hypervisor()?;
        hv.initialise(
            peb_addr,
            seed,
            page_size,
            outb_hdl,
            mem_access_hdl,
            max_execution_time,
            max_wait_for_cancellation,
        )
    }

    /// Get the stack pointer -- the value of the RSP register --
    /// the contained `Hypervisor` had
    pub fn orig_rsp(&self) -> Result<GuestPtr> {
        let hv = self
            .hv_opt
            .as_ref()
            .ok_or_else(|| anyhow!("no hypervisor present"))?
            .lock()
            .map_err(|_| anyhow!("couldn't lock hypervisor"))?;
        let orig_rsp = hv.orig_rsp()?;
        GuestPtr::try_from(RawPtr::from(orig_rsp))
    }

    /// Reset the stack pointer
    pub fn reset_rsp(&mut self, new_rsp: GuestPtr) -> Result<()> {
        let mut hv = self
            .hv_opt
            .as_mut()
            .ok_or_else(|| anyhow!("no hypervisor present"))?
            .lock()
            .map_err(|_| anyhow!("couldn't lock hypervisor"))?;
        hv.reset_rsp(new_rsp.absolute()?)
    }

    /// Dispatch a call from the host to the guest
    pub fn dispatch_call_from_host(&mut self, dispatch_func_addr: GuestPtr) -> Result<()> {
        let outb_hdl = self.outb_hdl.clone();
        let mem_access_hdl = self.mem_access_hdl.clone();
        let max_execution_time = self.max_execution_time;
        let max_wait_for_cancellation = self.max_wait_for_cancellation;
        let mut hv = self.get_hypervisor()?;
        let dispatch_raw_ptr = RawPtr::from(dispatch_func_addr.absolute()?);
        hv.dispatch_call_from_host(
            dispatch_raw_ptr,
            outb_hdl,
            mem_access_hdl,
            max_execution_time,
            max_wait_for_cancellation,
        )
    }
}

impl<'a> UninitializedSandbox<'a> {
    /// Set up the appropriate hypervisor for the platform
    ///
    /// TODO: remove this dead_code annotation after it's hooked up
    pub(super) fn set_up_hypervisor_partition(
        mgr: &mut SandboxMemoryManager,
    ) -> Result<Box<dyn Hypervisor>> {
        let mem_size = u64::try_from(mgr.shared_mem.mem_size())?;
        let rsp_ptr = {
            let rsp_u64 = mgr.set_up_hypervisor_partition(mem_size)?;
            let rsp_raw = RawPtr::from(rsp_u64);
            GuestPtr::try_from(rsp_raw)
        }?;
        let base_ptr = GuestPtr::try_from(Offset::from(0))?;
        let pml4_ptr = {
            let pml4_offset_u64 = u64::try_from(SandboxMemoryLayout::PML4_OFFSET)?;
            base_ptr.clone() + Offset::from(pml4_offset_u64)
        };
        let entrypoint_ptr = {
            let entrypoint_total_offset = mgr.load_addr.clone() + mgr.entrypoint_offset;
            GuestPtr::try_from(entrypoint_total_offset)
        }?;
        assert!(base_ptr == pml4_ptr);
        assert!(entrypoint_ptr > pml4_ptr);
        assert!(rsp_ptr > entrypoint_ptr);

        #[cfg(target_os = "linux")]
        {
            use crate::hypervisor::hypervisor_mem::HypervisorAddrs;
            use crate::hypervisor::{hyperv_linux, hyperv_linux::HypervLinuxDriver};
            use crate::hypervisor::{kvm, kvm::KVMDriver};

            if hyperv_linux::is_hypervisor_present().unwrap_or(false) {
                let guest_pfn = u64::try_from(SandboxMemoryLayout::BASE_ADDRESS >> 12)?;
                let host_addr = u64::try_from(mgr.shared_mem.base_addr())?;
                let addrs = HypervisorAddrs {
                    entrypoint: entrypoint_ptr.absolute()?,
                    guest_pfn,
                    host_addr,
                    mem_size,
                };
                let hv = HypervLinuxDriver::new(&addrs, rsp_ptr, pml4_ptr)?;
                Ok(Box::new(hv))
            } else if kvm::is_hypervisor_present().is_ok() {
                let host_addr = u64::try_from(mgr.shared_mem.base_addr())?;
                let hv = KVMDriver::new(
                    host_addr,
                    pml4_ptr.absolute()?,
                    mem_size,
                    entrypoint_ptr.absolute()?,
                    rsp_ptr.absolute()?,
                )?;
                Ok(Box::new(hv))
            } else {
                bail!("Linux platform detected, but neither KVM nor Linux HyperV detected")
            }
        }
        #[cfg(target_os = "windows")]
        {
            use crate::hypervisor::hyperv_windows::HypervWindowsDriver;
            use crate::hypervisor::windows_hypervisor_platform;
            if windows_hypervisor_platform::is_hypervisor_present().unwrap_or(false) {
                let source_addr = mgr.shared_mem.raw_ptr();
                let guest_base_addr = u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?;
                let hv = HypervWindowsDriver::new(
                    mgr.shared_mem.mem_size(),
                    source_addr,
                    guest_base_addr,
                    pml4_ptr.absolute()?,
                    entrypoint_ptr.absolute()?,
                    rsp_ptr.absolute()?,
                )?;
                Ok(Box::new(hv))
            } else {
                bail!("Windows platform detected but no hypervisor available")
            }
        }
    }
}

impl<'a> Debug for HypervisorWrapper<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HypervisorWrapper")
            .field("has_hypervisor", &self.hv_opt.is_some())
            .finish()
    }
}
