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

/// A container with convenience methods attached for an
/// `Option<Box<dyn Hypervisor>>`
pub struct HypervisorWrapper<'a> {
    hv: Option<Box<dyn Hypervisor>>,
    outb_hdl: OutBHandlerWrapper<'a>,
    mem_access_hdl: MemAccessHandlerWrapper<'a>,
}

pub trait HypervisorWrapperMgr<'a> {
    fn get_hypervisor_wrapper(&self) -> &HypervisorWrapper<'a>;
    fn get_hypervisor_wrapper_mut(&mut self) -> &mut HypervisorWrapper<'a>;
}

impl<'a> HypervisorWrapper<'a> {
    pub(super) fn new(
        hv: Option<Box<dyn Hypervisor>>,
        outb_hdl: OutBHandlerWrapper<'a>,
        mem_access_hdl: MemAccessHandlerWrapper<'a>,
    ) -> Self {
        Self {
            hv,
            outb_hdl,
            mem_access_hdl,
        }
    }
    pub fn get_hypervisor(&self) -> Result<&dyn Hypervisor> {
        self.hv
            .as_ref()
            .map(|h| h.as_ref())
            .ok_or(anyhow!("no hypervisor available for sandbox"))
    }

    pub fn get_hypervisor_mut(&mut self) -> Result<&mut dyn Hypervisor> {
        match self.hv.as_mut() {
            None => bail!("no hypervisor available for sandbox"),
            Some(h) => Ok(h.as_mut()),
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
        let hv = self.get_hypervisor_mut()?;
        hv.initialise(peb_addr, seed, page_size, outb_hdl, mem_access_hdl)
    }

    /// Get the stack pointer -- the value of the RSP register --
    /// the contained `Hypervisor` had
    pub fn orig_rsp(&self) -> Result<GuestPtr> {
        let hv = self.get_hypervisor()?;
        let orig_rsp = hv.orig_rsp()?;
        GuestPtr::try_from(RawPtr::from(orig_rsp))
    }

    pub fn reset_rsp(&mut self, new_rsp: GuestPtr) -> Result<()> {
        let hv = self.get_hypervisor_mut()?;
        hv.reset_rsp(new_rsp.absolute()?)
    }

    pub fn dispatch_call_from_host(&mut self, dispatch_func_addr: GuestPtr) -> Result<()> {
        let outb_hdl = self.outb_hdl.clone();
        let mem_access_hdl = self.mem_access_hdl.clone();
        let hv = self.get_hypervisor_mut()?;
        let dispatch_raw_ptr = RawPtr::from(dispatch_func_addr.absolute()?);
        hv.dispatch_call_from_host(dispatch_raw_ptr, outb_hdl, mem_access_hdl)
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
