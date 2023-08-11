use super::{
    handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper},
    Hypervisor, CR0_AM, CR0_ET, CR0_MP, CR0_NE, CR0_PE, CR0_PG, CR0_WP, CR4_OSFXSR, CR4_OSXMMEXCPT,
    CR4_PAE, EFER_LMA, EFER_LME,
};
use crate::mem::{layout::SandboxMemoryLayout, ptr::RawPtr};
use anyhow::{anyhow, bail, Result};
use kvm_bindings::{kvm_segment, kvm_userspace_memory_region};
use kvm_ioctls::{Cap::UserMemory, Kvm, VcpuExit, VcpuFd, VmFd};

/// Return `Ok(())` if the KVM API is available, or `Err` otherwise
pub(crate) fn is_hypervisor_present() -> Result<()> {
    let kvm = Kvm::new()?;
    let ver = kvm.get_api_version();
    if -1 == ver {
        bail!("KVM_GET_API_VERSION returned -1");
    } else if ver != 12 {
        bail!("KVM_GET_API_VERSION returned {}, expected 12", ver);
    }
    let cap_user_mem = kvm.check_extension(UserMemory);
    if !cap_user_mem {
        bail!("KVM_CAP_USER_MEMORY not supported");
    }
    Ok(())
}

/// A Hypervisor driver for KVM on Linux
#[derive(Debug)]
pub(crate) struct KVMDriver {
    // kvm and vm_fd are not used but must be present so they're properly
    // dropped.
    // prefix them with underscore so clippy doesn't complain they're unused
    _kvm: Kvm,
    _vm_fd: VmFd,
    vcpu_fd: VcpuFd,
    entrypoint: u64,
    rsp: u64,
}

impl KVMDriver {
    /// Create a new instance of a `KVMDriver`, with only control registers
    /// set. Standard registers will not be set, and `initialise` must
    /// be called to do so.
    ///
    /// TODO: when rust rewrite is complete, change `rsp` and `pml4_addr`
    /// params to be of type `GuestPtr`.
    pub(crate) fn new(
        host_addr: u64,
        pml4_addr: u64,
        mem_size: u64,
        entrypoint: u64,
        rsp: u64,
    ) -> Result<Self> {
        match is_hypervisor_present() {
            Ok(_) => (),
            Err(e) => bail!(e),
        };
        let kvm = Kvm::new()?;

        let vm_fd = kvm.create_vm_with_type(0)?;
        {
            // the address _inside the guest_ at which memory should start
            let guest_phys_addr = u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?;
            // set memory region
            let region = kvm_userspace_memory_region {
                slot: 0,
                // the starting address of memory in the guest
                guest_phys_addr,
                // the total size of guest memory
                memory_size: mem_size,
                // the address of the start of memory on the host
                userspace_addr: host_addr,
                flags: 0,
            };
            unsafe { vm_fd.set_user_memory_region(region) }
        }?;

        let mut vcpu_fd = vm_fd.create_vcpu(0)?;
        Self::set_sregs(&mut vcpu_fd, pml4_addr)?;

        Ok(Self {
            _kvm: kvm,
            _vm_fd: vm_fd,
            vcpu_fd,
            entrypoint,
            rsp,
        })
    }

    fn set_sreg_segment(seg: &mut kvm_segment, type_: u8, selector: u16) {
        seg.base = 0;
        seg.limit = 0xffffffff;
        seg.selector = selector;
        seg.present = 1;
        seg.type_ = type_;
        seg.dpl = 0;
        seg.db = 0;
        seg.s = 1;
        seg.l = 1;
        seg.g = 1;
    }

    fn set_sregs(vcpu_fd: &mut VcpuFd, pml4_addr: u64) -> Result<()> {
        // set up x86 memory segmentation registers.
        // these are primarily used in Hyperlight for purposes of
        // a setting up a memory hierarchy using page tables.
        //
        // for more on generally how this is done on x86 architectures, see
        // the below link:
        // https://en.wikipedia.org/wiki/X86_memory_segmentation
        //
        // some of this code in this function is inspired from the code
        // at the below link:
        //
        // https://github.com/rust-vmm/kvm-ioctls/blob/b0a258655e84c7ab2c50cbdae5324216fa530adb/src/lib.rs#L136-L140
        //
        let mut sregs = vcpu_fd.get_sregs()?;
        sregs.cr3 = pml4_addr;
        sregs.cr4 = CR4_PAE | CR4_OSFXSR | CR4_OSXMMEXCPT;
        sregs.cr0 = CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_WP | CR0_AM | CR0_PG;
        sregs.efer = EFER_LME | EFER_LMA;

        {
            // set up the code segment
            // https://en.wikipedia.org/wiki/Code_segment
            const CS_TYPE: u8 = 11;
            const CS_SELECTOR: u16 = 1 << 3;
            Self::set_sreg_segment(&mut sregs.cs, CS_TYPE, CS_SELECTOR);
        }
        {
            // set up the data segment
            // https://en.wikipedia.org/wiki/Data_segment
            const DS_TYPE: u8 = 3;
            const DS_SELECTOR: u16 = 2 << 3;
            Self::set_sreg_segment(&mut sregs.ds, DS_TYPE, DS_SELECTOR);
        }
        {
            // set up the extra segment
            const ES_TYPE: u8 = 3;
            const ES_SELECTOR: u16 = 2 << 3;
            Self::set_sreg_segment(&mut sregs.es, ES_TYPE, ES_SELECTOR);
        }
        {
            // set up the "F" segment. see the below link for a bit more
            // information.
            // https://en.wikipedia.org/wiki/I386#Architecture
            const FS_TYPE: u8 = 3;
            const FS_SELECTOR: u16 = 2 << 3;
            Self::set_sreg_segment(&mut sregs.fs, FS_TYPE, FS_SELECTOR);
        }
        {
            // set up the "G" segment. see the below link for a bit more
            // information.
            // https://en.wikipedia.org/wiki/I386#Architecture
            const GS_TYPE: u8 = 3;
            const GS_SELECTOR: u16 = 2 << 3;
            Self::set_sreg_segment(&mut sregs.gs, GS_TYPE, GS_SELECTOR);
        }
        {
            // set up the stack segment
            const SS_TYPE: u8 = 3;
            const SS_SELECTOR: u16 = 2 << 3;
            Self::set_sreg_segment(&mut sregs.ss, SS_TYPE, SS_SELECTOR);
        }

        vcpu_fd
            .set_sregs(&sregs)
            .map_err(|e| anyhow!("failed to set segment registers: {:?}", e))
    }

    fn handle_io(&self, port: u16, data: &[u8], outb_handle_fn: OutBHandlerWrapper) -> Result<()> {
        let mut regs = self.vcpu_fd.get_regs()?;
        let orig_rip = regs.rip;

        // the payload param for the outb_handle_fn is the the first byte
        // of the data array, casted to a u64. thus, we need to make sure
        // the data array has at least one u8, then convert that to a u64
        if data.is_empty() {
            bail!("no data was given in IO interrupt");
        } else {
            let payload_u64 = u64::from(data[0]);
            outb_handle_fn
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
                .call(port, payload_u64)?;
        }

        //TODO: +1 may be a hack, but it works for now, need to figure out
        // how to get the instruction length.
        regs.rip = orig_rip + 1;
        self.vcpu_fd.set_regs(&regs)?;
        Ok(())
    }
}

impl Hypervisor for KVMDriver {
    fn dispatch_call_from_host(
        &mut self,
        dispatch_func_addr: RawPtr,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
    ) -> Result<()> {
        // Move rip to the DispatchFunction pointer
        let mut regs = self.vcpu_fd.get_regs()?;
        regs.rip = dispatch_func_addr.into();
        self.vcpu_fd.set_regs(&regs)?;
        self.execute_until_halt(outb_handle_fn, mem_access_fn)
    }

    fn execute_until_halt(
        &mut self,
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
    ) -> Result<()> {
        loop {
            let run_res = self.vcpu_fd.run()?;
            match run_res {
                VcpuExit::Hlt => {
                    return Ok(());
                }
                VcpuExit::IoOut(port, data) => self.handle_io(port, data, outb_hdl.clone())?,
                VcpuExit::MmioRead(addr, data) => {
                    mem_access_hdl
                        .lock()
                        .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
                        .call()?;
                    bail!("MMIO read address {:#x}, data {:?}", addr, data);
                }
                VcpuExit::MmioWrite(addr, data) => {
                    mem_access_hdl
                        .lock()
                        .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
                        .call()?;
                    bail!("MMIO write address 0x{:x}, data {:?}", addr, data);
                }
                other => {
                    bail!("Unexpected KVM message {:?}", other)
                }
            }
        }
    }

    /// Implementation of initialise for Hypervisor trait.
    ///
    /// TODO: when Rust rewrite is complete, change `peb_addr` to be
    /// of type `GuestPtr`
    fn initialise(
        &mut self,
        peb_addr: RawPtr,
        seed: u64,
        page_size: u32,
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
    ) -> Result<()> {
        let mut regs = self.vcpu_fd.get_regs()?;
        regs.rip = self.entrypoint;
        regs.rsp = self.rsp;
        regs.rdx = seed;
        regs.r8 = u64::from(page_size);
        regs.rcx = peb_addr.into();
        regs.rflags = 0x2;
        self.vcpu_fd.set_regs(&regs)?;
        self.execute_until_halt(outb_hdl.clone(), mem_access_hdl.clone())
    }

    fn reset_rsp(&mut self, rsp: u64) -> Result<()> {
        let mut regs = self.vcpu_fd.get_regs()?;
        regs.rsp = rsp;
        self.vcpu_fd.set_regs(&regs).map_err(|e| {
            anyhow!(
                "reset_rsp: error setting new registers on KVM vCPU: {:?}",
                e
            )
        })
    }

    fn orig_rsp(&self) -> Result<u64> {
        Ok(self.rsp)
    }
}

#[cfg(test)]
pub(crate) mod test_cfg {
    use once_cell::sync::Lazy;
    use serde::Deserialize;

    pub(crate) static TEST_CONFIG: Lazy<TestConfig> =
        Lazy::new(|| match envy::from_env::<TestConfig>() {
            Ok(config) => config,
            Err(err) => panic!("error parsing config from env: {}", err),
        });
    pub(crate) static SHOULD_RUN_TEST: Lazy<bool> = Lazy::new(is_kvm_present);

    fn is_kvm_present() -> bool {
        println!(
            "KVM_SHOULD_BE_PRESENT is {}",
            TEST_CONFIG.kvm_should_be_present
        );
        let is_present = super::is_hypervisor_present().is_ok();
        if (is_present && !TEST_CONFIG.kvm_should_be_present)
            || (!is_present && TEST_CONFIG.kvm_should_be_present)
        {
            println!(
                "WARNING: KVM is-present returned {}, should be present is: {}",
                is_present, TEST_CONFIG.kvm_should_be_present
            );
        }
        is_present
    }
    fn kvm_should_be_present_default() -> bool {
        false
    }

    #[derive(Deserialize, Debug)]
    pub(crate) struct TestConfig {
        #[serde(default = "kvm_should_be_present_default")]
        // Set env var KVM_SHOULD_BE_PRESENT to require hyperv to be present for the tests.
        pub(crate) kvm_should_be_present: bool,
    }

    #[macro_export]
    macro_rules! should_run_kvm_linux_test {
        () => {{
            if !(*$crate::hypervisor::kvm::test_cfg::SHOULD_RUN_TEST) {
                println! {"Not Running KVM Test - SHOULD_RUN_TEST is false"}
                return;
            }
            println! {"Running Test - SHOULD_RUN_TEST is true"}
        }};
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::KVMDriver;
    use crate::{
        hypervisor::{
            handlers::{MemAccessHandler, OutBHandler},
            tests::test_initialise,
        },
        mem::ptr_offset::Offset,
    };
    use crate::{
        mem::{layout::SandboxMemoryLayout, ptr::GuestPtr},
        should_run_kvm_linux_test,
    };
    use anyhow::Result;

    #[test]
    fn test_init() {
        should_run_kvm_linux_test!();
        let outb_handler = {
            let func: Box<dyn FnMut(u16, u64) -> Result<()>> =
                Box::new(|_, _| -> Result<()> { Ok(()) });
            Arc::new(Mutex::new(OutBHandler::from(func)))
        };
        let mem_access_handler = {
            let func: Box<dyn FnMut() -> Result<()>> = Box::new(|| -> Result<()> { Ok(()) });
            Arc::new(Mutex::new(MemAccessHandler::from(func)))
        };
        test_initialise(
            outb_handler,
            mem_access_handler,
            |mgr, rsp_ptr, pml4_ptr| {
                let host_addr = u64::try_from(mgr.shared_mem.base_addr())?;
                let rsp = rsp_ptr.absolute()?;
                let entrypoint = {
                    let load_addr = mgr.load_addr.clone();
                    let load_offset_u64 =
                        u64::from(load_addr) - u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?;
                    let total_offset = Offset::from(load_offset_u64) + mgr.entrypoint_offset;
                    GuestPtr::try_from(total_offset)
                }?;

                let driver = KVMDriver::new(
                    host_addr,
                    pml4_ptr.absolute().unwrap(),
                    u64::try_from(mgr.shared_mem.mem_size()).unwrap(),
                    entrypoint.absolute().unwrap(),
                    rsp,
                )?;
                Ok(Box::new(driver))
            },
        )
        .unwrap();
    }
}
