use super::{
    handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper},
    HyperlightExit, Hypervisor, VirtualCPU, CR0_AM, CR0_ET, CR0_MP, CR0_NE, CR0_PE, CR0_PG, CR0_WP,
    CR4_OSFXSR, CR4_OSXMMEXCPT, CR4_PAE, EFER_LMA, EFER_LME,
};
use crate::hypervisor::hypervisor_handler::{
    HandlerMsg, HasCommunicationChannels, HasHypervisorState, HypervisorState, VCPUAction,
};
use crate::mem::memory_region::MemoryRegion;
use crate::mem::{
    memory_region::MemoryRegionFlags,
    ptr::{GuestPtr, RawPtr},
};
use crate::Result;
use crate::{log_then_return, new_error};
use crossbeam_channel::{Receiver, Sender};

use crossbeam::atomic::AtomicCell;
use kvm_bindings::{kvm_fpu, kvm_regs, kvm_userspace_memory_region, KVM_MEM_READONLY};
use kvm_ioctls::{Cap::UserMemory, Kvm, VcpuExit, VcpuFd, VmFd};
use std::sync::{Arc, Mutex, MutexGuard};
use std::{any::Any, convert::TryFrom};
use tracing::{instrument, Span};

/// Return `true` if the KVM API is available, version 12, and has UserMemory capability, or `false` otherwise
// TODO: Once CAPI is complete this does not need to be public
#[instrument(skip_all, parent = Span::current(), level = "Trace")]
pub fn is_hypervisor_present() -> bool {
    if let Ok(kvm) = Kvm::new() {
        let api_version = kvm.get_api_version();
        match api_version {
            version if version == 12 && kvm.check_extension(UserMemory) => true,
            12 => {
                log::info!("KVM does not have KVM_CAP_USER_MEMORY capability");
                false
            }
            version => {
                log::info!("KVM GET_API_VERSION returned {}, expected 12", version);
                false
            }
        }
    } else {
        log::info!("Error creating KVM object");
        false
    }
}

/// A Hypervisor driver for KVM on Linux
//TODO:(#1029) Once CAPI is complete this does not need to be public
#[derive(Debug)]
pub struct KVMDriver {
    _kvm: Kvm,
    _vm_fd: VmFd,
    vcpu_fd: VcpuFd,
    entrypoint: u64,
    orig_rsp: GuestPtr,
    mem_regions: Vec<MemoryRegion>,
    vcpu_action_transmitter: Option<crossbeam_channel::Sender<VCPUAction>>,
    vcpu_action_receiver: Option<crossbeam_channel::Receiver<VCPUAction>>,
    handler_message_receiver: Option<crossbeam_channel::Receiver<HandlerMsg>>,
    handler_message_transmitter: Option<crossbeam_channel::Sender<HandlerMsg>>,
    thread_id: Option<u64>,
    cancel_run_requested: Arc<AtomicCell<bool>>,
    run_cancelled: Arc<AtomicCell<bool>>,
    join_handle: Option<std::thread::JoinHandle<Result<()>>>,
    // ^^^ a Hypervisor's operations are executed on a Hypervisor Handler thread (i.e.,
    // separate from the main host thread). This is a handle to the Hypervisor Handler thread.
    state: Arc<Mutex<HypervisorState>>,
}

impl KVMDriver {
    /// Create a new instance of a `KVMDriver`, with only control registers
    /// set. Standard registers will not be set, and `initialise` must
    /// be called to do so.
    ///
    /// TODO: when rust rewrite is complete, change `rsp` and `pml4_addr`
    /// params to be of type `GuestPtr`.
    //TODO:(#1029) Once CAPI is complete this does not need to be public
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub fn new(
        mem_regions: Vec<MemoryRegion>,
        pml4_addr: u64,
        entrypoint: u64,
        rsp: u64,
    ) -> Result<Self> {
        if !is_hypervisor_present() {
            log_then_return!("KVM is not present");
        };
        let kvm = Kvm::new()?;

        let vm_fd = kvm.create_vm_with_type(0)?;

        mem_regions.iter().enumerate().try_for_each(|(i, region)| {
            let kvm_region = kvm_userspace_memory_region {
                slot: i as u32,
                guest_phys_addr: region.guest_region.start as u64,
                memory_size: (region.guest_region.end - region.guest_region.start) as u64,
                userspace_addr: region.host_region.start as u64,
                flags: match region.flags {
                    MemoryRegionFlags::READ => KVM_MEM_READONLY,
                    _ => 0, // normal, RWX
                },
            };
            unsafe { vm_fd.set_user_memory_region(kvm_region) }
        })?;

        let mut vcpu_fd = vm_fd.create_vcpu(0)?;
        Self::setup_inital_sregs(&mut vcpu_fd, pml4_addr)?;

        let rsp_gp = GuestPtr::try_from(RawPtr::from(rsp))?;
        Ok(Self {
            _kvm: kvm,
            _vm_fd: vm_fd,
            vcpu_fd,
            entrypoint,
            orig_rsp: rsp_gp,
            mem_regions,
            vcpu_action_transmitter: None,
            vcpu_action_receiver: None,
            handler_message_receiver: None,
            handler_message_transmitter: None,
            thread_id: None,
            cancel_run_requested: Arc::new(AtomicCell::new(false)),
            run_cancelled: Arc::new(AtomicCell::new(false)),
            join_handle: None,
            state: Arc::new(Mutex::new(HypervisorState::default())),
        })
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn setup_inital_sregs(vcpu_fd: &mut VcpuFd, pml4_addr: u64) -> Result<()> {
        // setup paging and IA-32e (64-bit) mode
        let mut sregs = vcpu_fd.get_sregs()?; // TODO start with default and set explicitly what we need
        sregs.cr3 = pml4_addr;
        sregs.cr4 = CR4_PAE | CR4_OSFXSR | CR4_OSXMMEXCPT;
        sregs.cr0 = CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_WP | CR0_AM | CR0_PG;
        sregs.efer = EFER_LME | EFER_LMA;
        sregs.cs.l = 1; // required for 64-bit mode
        vcpu_fd.set_sregs(&sregs)?;
        Ok(())
    }
}

impl Hypervisor for KVMDriver {
    /// Implementation of initialise for Hypervisor trait.
    ///
    /// TODO: when Rust rewrite is complete, change `peb_addr` to be
    /// of type `GuestPtr`
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn initialise(
        &mut self,
        peb_addr: RawPtr,
        seed: u64,
        page_size: u32,
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
    ) -> Result<()> {
        let regs = kvm_regs {
            rip: self.entrypoint,
            rsp: self.orig_rsp.absolute()?,

            // function args
            rcx: peb_addr.into(),
            rdx: seed,
            r8: page_size.into(),
            r9: self.get_max_log_level().into(),

            ..Default::default()
        };
        self.vcpu_fd.set_regs(&regs)?;

        VirtualCPU::run(self.as_mut_hypervisor(), outb_hdl, mem_access_hdl)?;

        // reset RSP to what it was before initialise
        self.vcpu_fd.set_regs(&kvm_regs {
            rsp: self.orig_rsp.absolute()?,
            ..Default::default()
        })?;
        Ok(())
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn dispatch_call_from_host(
        &mut self,
        dispatch_func_addr: RawPtr,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
    ) -> Result<()> {
        // Reset general purpose registers except RSP, then set RIP
        let rsp_before = self.vcpu_fd.get_regs()?.rsp;
        let regs = kvm_regs {
            rip: dispatch_func_addr.into(),
            rsp: rsp_before,
            ..Default::default()
        };
        self.vcpu_fd.set_regs(&regs)?;

        // reset fpu state
        self.vcpu_fd.set_fpu(&kvm_fpu::default())?;

        // run
        VirtualCPU::run(self.as_mut_hypervisor(), outb_handle_fn, mem_access_fn)?;

        // reset RSP to what it was before function call
        self.vcpu_fd.set_regs(&kvm_regs {
            rsp: rsp_before,
            ..Default::default()
        })?;
        Ok(())
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn handle_io(
        &mut self,
        port: u16,
        data: Vec<u8>,
        rip: u64,
        instruction_length: u64,
        outb_handle_fn: OutBHandlerWrapper,
    ) -> Result<()> {
        let mut regs = self.vcpu_fd.get_regs()?;

        // The payload param for the outb_handle_fn is the first byte
        // of the data array cast to an u64. Thus, we need to make sure
        // the data array has at least one u8, then convert that to an u64
        if data.is_empty() {
            log_then_return!("no data was given in IO interrupt");
        } else {
            let payload_u64 = u64::from(data[0]);
            outb_handle_fn
                .lock()
                .map_err(|e| new_error!("Error Locking {}", e))?
                .call(port, payload_u64)?;
        }

        // update rip
        regs.rip = rip + instruction_length;
        self.vcpu_fd.set_regs(&regs)?;
        Ok(())
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn run(&mut self) -> Result<HyperlightExit> {
        let exit_reason = self.vcpu_fd.run();
        let result = match exit_reason {
            Ok(VcpuExit::Hlt) => HyperlightExit::Halt(),
            Ok(VcpuExit::IoOut(port, data)) => {
                let regs = self.vcpu_fd.get_regs()?;
                let rip = regs.rip;
                //TODO: 1 may be a hack, but it works for now, need to figure out
                // how to get the instruction length.
                let instruction_length = 1;

                HyperlightExit::IoOut(port, data.to_vec(), rip, instruction_length)
            }
            Ok(VcpuExit::MmioRead(addr, _)) => {
                let gpa = addr as usize;
                match self.get_memory_access_violation(
                    gpa,
                    &self.mem_regions,
                    MemoryRegionFlags::READ,
                ) {
                    Some(access_violation_exit) => access_violation_exit,
                    None => HyperlightExit::Mmio(addr),
                }
            }
            Ok(VcpuExit::MmioWrite(addr, _)) => {
                let gpa = addr as usize;
                match self.get_memory_access_violation(
                    gpa,
                    &self.mem_regions,
                    MemoryRegionFlags::WRITE,
                ) {
                    Some(access_violation_exit) => access_violation_exit,
                    None => HyperlightExit::Mmio(addr),
                }
            }
            Err(e) => match e.errno() {
                // we send a signal to the thread to cancel execution this results in EINTR being returned by KVM so we return Cancelled
                libc::EINTR => HyperlightExit::Cancelled(),
                libc::EAGAIN => HyperlightExit::Retry(),
                _ => {
                    log_then_return!("Error running VCPU {:?}", e);
                }
            },
            Ok(other) => HyperlightExit::Unknown(format!("Unexpected KVM Exit {:?}", other)),
        };
        Ok(result)
    }

    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn as_mut_hypervisor(&mut self) -> &mut dyn Hypervisor {
        self as &mut dyn Hypervisor
    }

    fn set_handler_join_handle(&mut self, handle: std::thread::JoinHandle<Result<()>>) {
        self.join_handle = Some(handle);
    }

    fn get_mut_handler_join_handle(&mut self) -> &mut Option<std::thread::JoinHandle<Result<()>>> {
        &mut self.join_handle
    }

    fn set_thread_id(&mut self, thread_id: u64) {
        log::debug!("Setting thread id to {}", thread_id);
        self.thread_id = Some(thread_id);
    }

    fn get_thread_id(&self) -> u64 {
        self.thread_id
            .expect("Hypervisor hasn't been initialized yet, missing thread ID")
    }

    fn set_termination_status(&mut self, value: bool) {
        log::debug!("Setting termination status to {}", value);
        self.cancel_run_requested.store(value);
    }

    fn get_termination_status(&self) -> Arc<AtomicCell<bool>> {
        self.cancel_run_requested.clone()
    }

    fn get_run_cancelled(&self) -> Arc<AtomicCell<bool>> {
        self.run_cancelled.clone()
    }

    fn set_run_cancelled(&self, value: bool) {
        log::debug!("Setting run cancelled to {}", value);
        self.run_cancelled.store(value);
    }
}

impl HasCommunicationChannels for KVMDriver {
    fn get_to_handler_tx(&self) -> Sender<VCPUAction> {
        self.vcpu_action_transmitter.clone().unwrap()
    }
    fn set_to_handler_tx(&mut self, tx: Sender<VCPUAction>) {
        self.vcpu_action_transmitter = Some(tx);
    }
    fn drop_to_handler_tx(&mut self) {
        self.vcpu_action_transmitter = None;
    }

    fn get_from_handler_rx(&self) -> Receiver<HandlerMsg> {
        self.handler_message_receiver.clone().unwrap()
    }
    fn set_from_handler_rx(&mut self, rx: Receiver<HandlerMsg>) {
        self.handler_message_receiver = Some(rx);
    }

    fn get_from_handler_tx(&self) -> Sender<HandlerMsg> {
        self.handler_message_transmitter.clone().unwrap()
    }
    fn set_from_handler_tx(&mut self, tx: Sender<HandlerMsg>) {
        self.handler_message_transmitter = Some(tx);
    }

    fn set_to_handler_rx(&mut self, rx: Receiver<VCPUAction>) {
        self.vcpu_action_receiver = Some(rx);
    }
    fn get_to_handler_rx(&self) -> Receiver<VCPUAction> {
        self.vcpu_action_receiver.clone().unwrap()
    }
}

impl HasHypervisorState for KVMDriver {
    fn get_state_lock(&self) -> Result<MutexGuard<HypervisorState>> {
        let state_mutex = Arc::as_ref(&self.state);

        Ok(state_mutex.lock()?)
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
        let is_present = super::is_hypervisor_present();
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
    use crate::Result;
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

    #[test]
    fn test_init() {
        should_run_kvm_linux_test!();
        let outb_handler = {
            let func: Box<dyn FnMut(u16, u64) -> Result<()> + Send> =
                Box::new(|_, _| -> Result<()> { Ok(()) });
            Arc::new(Mutex::new(OutBHandler::from(func)))
        };
        let mem_access_handler = {
            let func: Box<dyn FnMut() -> Result<()> + Send> = Box::new(|| -> Result<()> { Ok(()) });
            Arc::new(Mutex::new(MemAccessHandler::from(func)))
        };
        test_initialise(
            outb_handler,
            mem_access_handler,
            |mgr, rsp_ptr, pml4_ptr| {
                let rsp = rsp_ptr.absolute()?;
                let entrypoint = {
                    let load_addr = mgr.load_addr.clone();
                    let load_offset_u64 =
                        u64::from(load_addr) - u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?;
                    let total_offset = Offset::from(load_offset_u64) + mgr.entrypoint_offset;
                    GuestPtr::try_from(total_offset)
                }?;

                let driver = KVMDriver::new(
                    mgr.layout.get_memory_regions(&mgr.shared_mem),
                    pml4_ptr.absolute().unwrap(),
                    entrypoint.absolute().unwrap(),
                    rsp,
                )?;
                Ok(Box::new(driver))
            },
        )
        .unwrap();
    }
}
