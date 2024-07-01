use super::{
    handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper},
    Hypervisor, VirtualCPU, CR0_AM, CR0_ET, CR0_MP, CR0_NE, CR0_PE, CR0_PG, CR0_WP, CR4_OSFXSR,
    CR4_OSXMMEXCPT, CR4_PAE, EFER_LMA, EFER_LME,
};

use crate::hypervisor::hypervisor_handler::{
    HandlerMsg, HasCommunicationChannels, HasHypervisorState, HypervisorState, VCPUAction,
};
use crate::mem::memory_region::{MemoryRegion, MemoryRegionFlags};
use crate::{hypervisor::HyperlightExit, mem::ptr::RawPtr};
use crate::{log_then_return, mem::ptr::GuestPtr, new_error, Result};
use crossbeam::atomic::AtomicCell;
use crossbeam_channel::{Receiver, Sender};
use log::error;
use mshv_bindings::{
    hv_message, hv_message_type, hv_message_type_HVMSG_GPA_INTERCEPT,
    hv_message_type_HVMSG_UNMAPPED_GPA, hv_message_type_HVMSG_X64_HALT,
    hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT, hv_register_assoc,
    hv_register_name_HV_X64_REGISTER_CR0, hv_register_name_HV_X64_REGISTER_CR3,
    hv_register_name_HV_X64_REGISTER_CR4, hv_register_name_HV_X64_REGISTER_CS,
    hv_register_name_HV_X64_REGISTER_EFER, hv_register_name_HV_X64_REGISTER_RIP, hv_register_value,
    hv_u128, mshv_user_mem_region, FloatingPointUnit, StandardRegisters,
};
use mshv_ioctls::{Mshv, VcpuFd, VmFd};
use std::any::Any;
use std::sync::{Arc, Mutex, MutexGuard};
use tracing::{instrument, Span};

/// Determine whether the HyperV for Linux hypervisor API is present
/// and functional.
#[instrument(skip_all, parent = Span::current(), level = "Trace")]
//TODO:(#1029) Once CAPI is complete this does not need to be public
pub fn is_hypervisor_present() -> bool {
    match Mshv::open_with_cloexec(true) {
        Ok(fd) => {
            unsafe {
                libc::close(fd);
            } // must explicitly close fd to avoid a leak
            true
        }
        Err(e) => {
            log::info!("Error creating MSHV object: {:?}", e);
            false
        }
    }
}

/// A Hypervisor driver for HyperV-on-Linux. This hypervisor is often
/// called the Microsoft Hypervisor (MSHV)
//TODO:(#1029) Once CAPI is complete this does not need to be public
#[derive(Debug)]
pub struct HypervLinuxDriver {
    _mshv: Mshv,
    vm_fd: VmFd,
    vcpu_fd: VcpuFd,
    entrypoint: u64,
    mem_regions: Vec<MemoryRegion>,
    orig_rsp: GuestPtr,
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

impl HypervLinuxDriver {
    /// Create a new `HypervLinuxDriver`, complete with all registers
    /// set up to execute a Hyperlight binary inside a HyperV-powered
    /// sandbox on Linux.
    ///
    /// While registers are set up, they will not have been applied to
    /// the underlying virtual CPU after this function returns. Call the
    /// `apply_registers` method to do that, or more likely call
    /// `initialise` to do it for you.
    //TODO:(#1029) Once CAPI is complete this does not need to be public
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    pub fn new(
        mem_regions: Vec<MemoryRegion>,
        entrypoint_ptr: GuestPtr,
        rsp_ptr: GuestPtr,
        pml4_ptr: GuestPtr,
    ) -> Result<Self> {
        if !is_hypervisor_present() {
            log_then_return!("Hyper-V is not present on this system");
        }
        let mshv = Mshv::new()?;
        let pr = Default::default();
        let vm_fd = mshv.create_vm_with_config(&pr)?;
        let mut vcpu_fd = vm_fd.create_vcpu(0)?;

        mem_regions.iter().try_for_each(|region| {
            let mshv_region = region.to_owned().into();
            vm_fd.map_user_memory(mshv_region)
        })?;

        Self::setup_initial_sregs(&mut vcpu_fd, pml4_ptr.absolute()?)?;

        Ok(Self {
            _mshv: mshv,
            vm_fd,
            vcpu_fd,
            mem_regions,
            entrypoint: entrypoint_ptr.absolute()?,
            orig_rsp: rsp_ptr,
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
    fn setup_initial_sregs(vcpu: &mut VcpuFd, pml4_addr: u64) -> Result<()> {
        vcpu.set_reg(&[
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_CR3,
                value: hv_register_value { reg64: pml4_addr },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_CR4,
                value: hv_register_value {
                    reg64: CR4_PAE | CR4_OSFXSR | CR4_OSXMMEXCPT,
                },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_CR0,
                value: hv_register_value {
                    reg64: CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_WP | CR0_AM | CR0_PG,
                },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_EFER,
                value: hv_register_value {
                    reg64: EFER_LME | EFER_LMA,
                },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_CS,
                value: hv_register_value {
                    reg128: hv_u128 {
                        low_part: 0,
                        high_part: 0xa09b0008ffffffff,
                    },
                },
                ..Default::default()
            },
        ])?;
        Ok(())
    }
}

impl Hypervisor for HypervLinuxDriver {
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn initialise(
        &mut self,
        peb_addr: RawPtr,
        seed: u64,
        page_size: u32,
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
    ) -> Result<()> {
        let regs = StandardRegisters {
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
        self.vcpu_fd.set_regs(&StandardRegisters {
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
        let regs = StandardRegisters {
            rip: dispatch_func_addr.into(),
            rsp: rsp_before,
            ..Default::default()
        };
        self.vcpu_fd.set_regs(&regs)?;

        // reset fpu state
        self.vcpu_fd.set_fpu(&FloatingPointUnit::default())?;

        // run
        VirtualCPU::run(self.as_mut_hypervisor(), outb_handle_fn, mem_access_fn)?;

        // reset RSP to what it was before function call
        self.vcpu_fd.set_regs(&StandardRegisters {
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
        let payload = data[..8].try_into()?;
        outb_handle_fn
            .lock()
            .map_err(|e| new_error!("Error Locking {}", e))?
            .call(port, u64::from_le_bytes(payload))?;

        // update rip
        self.vcpu_fd.set_reg(&[hv_register_assoc {
            name: hv_register_name_HV_X64_REGISTER_RIP,
            value: hv_register_value {
                reg64: rip + instruction_length,
            },
            ..Default::default()
        }])?;
        Ok(())
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn run(&mut self) -> Result<super::HyperlightExit> {
        const HALT_MESSAGE: hv_message_type = hv_message_type_HVMSG_X64_HALT;
        const IO_PORT_INTERCEPT_MESSAGE: hv_message_type =
            hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT;
        const UNMAPPED_GPA_MESSAGE: hv_message_type = hv_message_type_HVMSG_UNMAPPED_GPA;
        const INVALID_GPA_ACCESS_MESSAGE: hv_message_type = hv_message_type_HVMSG_GPA_INTERCEPT;

        let hv_message: hv_message = Default::default();
        let result = match &self.vcpu_fd.run(hv_message) {
            Ok(m) => match m.header.message_type {
                HALT_MESSAGE => HyperlightExit::Halt(),
                IO_PORT_INTERCEPT_MESSAGE => {
                    let io_message = m.to_ioport_info()?;
                    let port_number = io_message.port_number;
                    let rip = io_message.header.rip;
                    let rax = io_message.rax;
                    let instruction_length = io_message.header.instruction_length() as u64;

                    HyperlightExit::IoOut(
                        port_number,
                        rax.to_le_bytes().to_vec(),
                        rip,
                        instruction_length,
                    )
                }
                UNMAPPED_GPA_MESSAGE => {
                    let mimo_message = m.to_memory_info()?;
                    let addr = mimo_message.guest_physical_address;
                    HyperlightExit::Mmio(addr)
                }
                INVALID_GPA_ACCESS_MESSAGE => {
                    let mimo_message = m.to_memory_info()?;
                    let gpa = mimo_message.guest_physical_address;
                    let access_info = MemoryRegionFlags::try_from(mimo_message)?;

                    match self.get_memory_access_violation(
                        gpa as usize,
                        &self.mem_regions,
                        access_info,
                    ) {
                        Some(access_info_violation) => access_info_violation,
                        None => HyperlightExit::Mmio(gpa),
                    }
                }
                other => {
                    log_then_return!("unknown Hyper-V run message type {:?}", other);
                }
            },
            Err(e) => match e.errno() {
                // we send a signal to the thread to cancel execution this results in EINTR being returned by KVM so we return Cancelled
                libc::EINTR => HyperlightExit::Cancelled(),
                libc::EAGAIN => HyperlightExit::Retry(),
                _ => {
                    log_then_return!("Error running VCPU {:?}", e);
                }
            },
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

impl HasCommunicationChannels for HypervLinuxDriver {
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

impl HasHypervisorState for HypervLinuxDriver {
    fn get_state_lock(&self) -> Result<MutexGuard<HypervisorState>> {
        let state_mutex = Arc::as_ref(&self.state);

        Ok(state_mutex.lock()?)
    }
}

impl Drop for HypervLinuxDriver {
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn drop(&mut self) {
        for region in &self.mem_regions {
            let mshv_region: mshv_user_mem_region = region.to_owned().into();
            match self.vm_fd.unmap_user_memory(mshv_region) {
                Ok(_) => (),
                Err(e) => error!("Failed to unmap user memory in HyperVOnLinux ({:?})", e),
            }
        }
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
    pub(crate) static SHOULD_RUN_TEST: Lazy<bool> = Lazy::new(is_hyperv_present);

    fn is_hyperv_present() -> bool {
        println!(
            "HYPERV_SHOULD_BE_PRESENT is {}",
            TEST_CONFIG.hyperv_should_be_present
        );
        let is_present = super::is_hypervisor_present();
        if (is_present && !TEST_CONFIG.hyperv_should_be_present)
            || (!is_present && TEST_CONFIG.hyperv_should_be_present)
        {
            panic!(
                "WARNING Hyper-V is present returned  {}, should be present is: {}",
                is_present, TEST_CONFIG.hyperv_should_be_present
            );
        }
        is_present
    }

    fn hyperv_should_be_present_default() -> bool {
        false
    }

    #[derive(Deserialize, Debug)]
    pub(crate) struct TestConfig {
        #[serde(default = "hyperv_should_be_present_default")]
        // Set env var HYPERV_SHOULD_BE_PRESENT to require hyperv to be present for the tests.
        pub(crate) hyperv_should_be_present: bool,
    }

    #[macro_export]
    macro_rules! should_run_hyperv_linux_test {
        () => {{
            if !(*SHOULD_RUN_TEST) {
                println! {"Not Running Test SHOULD_RUN_TEST is false"}
                return;
            }
            println! {"Running Test SHOULD_RUN_TEST is true"}
        }};
    }
}

#[cfg(test)]
mod tests {
    use super::test_cfg::{SHOULD_RUN_TEST, TEST_CONFIG};
    use super::*;
    use crate::mem::memory_region::MemoryRegionVecBuilder;
    use crate::{mem::shared_mem::SharedMemory, should_run_hyperv_linux_test};

    #[rustfmt::skip]
    const CODE: [u8; 12] = [
        0xba, 0xf8, 0x03, /* mov $0x3f8, %dx */
        0x00, 0xd8, /* add %bl, %al */
        0x04, b'0', /* add $'0', %al */
        0xee, /* out %al, (%dx) */
        /* send a 0 to indicate we're done */
        0xb0, b'\0', /* mov $'\0', %al */
        0xee, /* out %al, (%dx) */
        0xf4, /* HLT */
    ];

    fn shared_mem_with_code(
        code: &[u8],
        mem_size: usize,
        load_offset: usize,
    ) -> Result<Box<SharedMemory>> {
        if load_offset > mem_size {
            log_then_return!(
                "code load offset ({}) > memory size ({})",
                load_offset,
                mem_size
            );
        }
        let mut shared_mem = SharedMemory::new(mem_size)?;
        shared_mem.copy_from_slice(code, load_offset)?;
        Ok(Box::new(shared_mem))
    }

    #[test]
    fn is_hypervisor_present() {
        let result = super::is_hypervisor_present();
        assert_eq!(result, TEST_CONFIG.hyperv_should_be_present);
    }

    #[test]
    fn create_driver() {
        should_run_hyperv_linux_test!();
        const MEM_SIZE: usize = 0x3000;
        let gm = shared_mem_with_code(CODE.as_slice(), MEM_SIZE, 0).unwrap();
        let rsp_ptr = GuestPtr::try_from(0).unwrap();
        let pml4_ptr = GuestPtr::try_from(0).unwrap();
        let entrypoint_ptr = GuestPtr::try_from(0).unwrap();
        let mut regions = MemoryRegionVecBuilder::new(0, gm.base_addr());
        regions.push_page_aligned(
            MEM_SIZE,
            MemoryRegionFlags::READ | MemoryRegionFlags::WRITE | MemoryRegionFlags::EXECUTE,
        );
        super::HypervLinuxDriver::new(regions.build(), entrypoint_ptr, rsp_ptr, pml4_ptr).unwrap();
    }
}
