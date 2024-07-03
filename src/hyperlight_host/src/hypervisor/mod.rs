use crate::error::HyperlightError::ExecutionCanceledByHost;
#[cfg(target_os = "linux")]
use crate::error::HyperlightError::HostFailedToCancelGuestExecutionSendingSignals;
#[cfg(target_os = "windows")]
use crate::hypervisor::hypervisor_handler::PARTITION_HANDLE;
use crate::hypervisor::metrics::HypervisorMetric::NumberOfCancelledGuestExecutions;
use crate::mem::memory_region::MemoryRegion;
use crate::mem::memory_region::MemoryRegionFlags;
use crate::new_error;
use crate::HyperlightError;
use crate::Result;
use crate::{int_counter_inc, log_then_return};
#[cfg(target_os = "linux")]
use libc::{pthread_kill, ESRCH};
use log::error;
#[cfg(target_os = "linux")]
use log::info;
use tracing::{instrument, Span};
#[cfg(target_os = "linux")]
use vmm_sys_util::signal::SIGRTMIN;
#[cfg(target_os = "windows")]
use windows::Win32::System::Hypervisor::WHvCancelRunVirtualProcessor;

/// Handlers for Hypervisor custom logic
pub mod handlers;
/// HyperV-on-linux functionality
#[cfg(target_os = "linux")]
pub mod hyperv_linux;
#[cfg(target_os = "windows")]
/// Hyperv-on-windows functionality
pub(crate) mod hyperv_windows;
pub(crate) mod hypervisor_handler;
#[cfg(target_os = "linux")]
/// Functionality to manipulate KVM-based virtual machines
pub mod kvm;
/// Metric definitions for Hypervisor module.
mod metrics;
#[cfg(target_os = "windows")]
/// Hyperlight Surrogate Process
pub(crate) mod surrogate_process;
#[cfg(target_os = "windows")]
/// Hyperlight Surrogate Process
pub(crate) mod surrogate_process_manager;
/// WindowsHypervisorPlatform utilities
#[cfg(target_os = "windows")]
pub(crate) mod windows_hypervisor_platform;
/// Safe wrappers around windows types like `PSTR`
#[cfg(target_os = "windows")]
mod wrappers;

use self::handlers::{
    MemAccessHandlerCaller, MemAccessHandlerWrapper, OutBHandlerCaller, OutBHandlerWrapper,
};
use crate::hypervisor::hypervisor_handler::HasCommunicationChannels;
use crate::mem::ptr::RawPtr;
use crossbeam::atomic::AtomicCell;
use std::{
    any::Any,
    fmt::Debug,
    sync::{Arc, Mutex},
    time::Duration,
};

pub(crate) const CR4_PAE: u64 = 1 << 5;
pub(crate) const CR4_OSFXSR: u64 = 1 << 9;
pub(crate) const CR4_OSXMMEXCPT: u64 = 1 << 10;
pub(crate) const CR0_PE: u64 = 1;
pub(crate) const CR0_MP: u64 = 1 << 1;
pub(crate) const CR0_ET: u64 = 1 << 4;
pub(crate) const CR0_NE: u64 = 1 << 5;
pub(crate) const CR0_WP: u64 = 1 << 16;
pub(crate) const CR0_AM: u64 = 1 << 18;
pub(crate) const CR0_PG: u64 = 1 << 31;
pub(crate) const EFER_LME: u64 = 1 << 8;
pub(crate) const EFER_LMA: u64 = 1 << 10;

/// These are the generic exit reasons that we can handle from a Hypervisor the Hypervisors run method is responsible for mapping from
/// the hypervisor specific exit reasons to these generic ones
pub enum HyperlightExit {
    /// The vCPU has halted
    Halt(),
    /// The vCPU has issued a write to the given port with the given value
    IoOut(u16, Vec<u8>, u64, u64),
    /// The vCPU has attempted to read or write from an unmapped address
    Mmio(u64),
    /// The vCPU tried to access memory but was missing the required permissions
    AccessViolation(u64, MemoryRegionFlags, MemoryRegionFlags),
    /// The vCPU execution has been cancelled
    Cancelled(),
    /// The vCPU has exited for a reason that is not handled by Hyperlight
    Unknown(String),
    /// The operation should be retried, for example this can happen on Linux where a call to run the CPU can return EAGAIN
    Retry(),
}

/// A common set of hypervisor functionality
pub trait Hypervisor: Debug + Sync + Send + HasCommunicationChannels {
    /// Initialise the internally stored vCPU with the given PEB address and
    /// random number seed, then run it until a HLT instruction.
    #[allow(clippy::too_many_arguments)]
    fn initialise(
        &mut self,
        peb_addr: RawPtr,
        seed: u64,
        page_size: u32,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
    ) -> Result<()>;

    /// Dispatch a call from the host to the guest using the given pointer
    /// to the dispatch function _in the guest's address space_.
    ///
    /// Do this by setting the instruction pointer to `dispatch_func_addr`
    /// and then running the execution loop until a halt instruction.
    ///
    /// Returns `Ok` if the call succeeded, and an `Err` if it failed
    fn dispatch_call_from_host(
        &mut self,
        dispatch_func_addr: RawPtr,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
    ) -> Result<()>;

    /// Handle an IO exit from the internally stored vCPU.
    fn handle_io(
        &mut self,
        port: u16,
        data: Vec<u8>,
        rip: u64,
        instruction_length: u64,
        outb_handle_fn: OutBHandlerWrapper,
    ) -> Result<()>;

    /// Run the vCPU
    fn run(&mut self) -> Result<HyperlightExit>;

    /// Returns a Some(HyperlightExit::AccessViolation(..)) if the given gpa doesn't have
    /// access its corresponding region. Returns None otherwise, or if the region is not found.
    fn get_memory_access_violation(
        &self,
        gpa: usize,
        mem_regions: &[MemoryRegion],
        access_info: MemoryRegionFlags,
    ) -> Option<HyperlightExit> {
        // find the region containing the given gpa
        let region = mem_regions
            .iter()
            .find(|region| region.guest_region.contains(&gpa));

        if let Some(region) = region {
            if !region.flags.contains(access_info) {
                return Some(HyperlightExit::AccessViolation(
                    gpa as u64,
                    access_info,
                    region.flags,
                ));
            }
        }
        None
    }

    /// Set up To/From channels for the Hypervisor handler
    fn setup_hypervisor_handler_communication_channels(&mut self) {
        let (to_handler_tx, to_handler_rx) = crossbeam_channel::unbounded();
        let (from_handler_tx, from_handler_rx) = crossbeam_channel::unbounded();

        self.set_to_handler_tx(to_handler_tx.clone());

        self.set_to_handler_rx(to_handler_rx.clone());
        self.set_from_handler_rx(from_handler_rx.clone());
        self.set_from_handler_tx(from_handler_tx.clone());
    }

    /// Get the logging level to pass to the guest entrypoint
    fn get_max_log_level(&self) -> u32 {
        log::max_level() as u32
    }

    /// Allow the hypervisor to be downcast
    fn as_any(&self) -> &dyn Any;

    /// get a mutable trait object from self
    fn as_mut_hypervisor(&mut self) -> &mut dyn Hypervisor;

    ///
    /// ###### THREADING ######
    ///

    /// Set the JoinHandle for the Hypervisor handler
    fn set_handler_join_handle(&mut self, handle: std::thread::JoinHandle<Result<()>>);

    /// Get the JoinHandle for the Hypervisor handler
    fn get_mut_handler_join_handle(&mut self) -> &mut Option<std::thread::JoinHandle<Result<()>>>;

    /// Set the thread ID the Hypervisor is running on
    #[cfg(target_os = "linux")]
    fn set_thread_id(&mut self, thread_id: u64);

    /// Get the thread ID the Hypervisor is running on
    #[cfg(target_os = "linux")]
    fn get_thread_id(&self) -> u64;

    /// Request termination of the Hypervisor
    fn set_termination_status(&mut self, value: bool);

    /// Get termination status of the Hypervisor
    fn get_termination_status(&self) -> Arc<AtomicCell<bool>>;

    /// On Linux, to stop the execution we need to signal the thread to cause an EINTR error on
    /// the underlying VM run call. The EINTR error is then handled by the run method, and it
    /// returns a `HyperlightExit::Cancelled` to indicate that the execution was cancelled.
    ///
    /// So, the way we are going to solve this for now is to get the `p_thread_t` via a libc call
    /// in the spawned thread and then use an `AtomicCell` to make it available to host thread.
    /// Then, if we need to signal the thread to cancel the execution, we can get the `thread_id`
    /// from the `AtomicCell` and call `pthread_kill` on it. This function gets the confirmation that
    /// `request_termination` was successful and the thread was signalled to cancel the execution.
    #[cfg(target_os = "linux")]
    fn get_run_cancelled(&self) -> Arc<AtomicCell<bool>>;

    /// Set cancellation confirmation
    #[cfg(target_os = "linux")]
    fn set_run_cancelled(&self, value: bool);
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
pub(crate) fn terminate_execution(
    timeout: Duration,
    cancel_run_requested: Arc<AtomicCell<bool>>,
    #[cfg(target_os = "linux")] run_cancelled: Arc<AtomicCell<bool>>,
    #[cfg(target_os = "linux")] thread_id: u64,
    #[cfg(target_os = "linux")] timeout_wait_to_cancel: Duration,
) -> Result<()> {
    error!(
        "Execution timed out after {} milliseconds , cancelling execution",
        timeout.as_millis()
    );

    cancel_run_requested.store(true);

    #[cfg(target_os = "linux")]
    {
        if thread_id == u64::MAX {
            log_then_return!("Failed to get thread id to signal thread");
        }
        let mut count: i32 = 0;
        // We need to send the signal multiple times in case the thread was between checking if it should be cancelled
        // and entering the run loop

        // we cannot do this forever (if the thread is calling a host function that never returns we will sit here forever)
        // so use the timeout_wait_to_cancel to limit the number of iterations

        let number_of_iterations = timeout_wait_to_cancel.as_micros() / 500;

        while !run_cancelled.load() {
            count += 1;

            if count > number_of_iterations.try_into().unwrap() {
                break;
            }

            info!(
                "Sending signal to thread {} iteration: {}",
                thread_id, count
            );

            let ret = unsafe { pthread_kill(thread_id, SIGRTMIN()) };
            // We may get ESRCH if we try to signal a thread that has already exited
            if ret < 0 && ret != ESRCH {
                log_then_return!("error {} calling pthread_kill", ret);
            }
            std::thread::sleep(Duration::from_micros(500));
        }
        if !run_cancelled.load() {
            log_then_return!(HostFailedToCancelGuestExecutionSendingSignals(count));
        }
    }
    #[cfg(target_os = "windows")]
    {
        unsafe {
            PARTITION_HANDLE.with(|elem| -> Result<()> {
                let partition_handle = elem.lock().unwrap();
                WHvCancelRunVirtualProcessor(*partition_handle, 0, 0)
                    .map_err(|e| new_error!("Failed to cancel guest execution {:?}", e))?;

                Ok(())
            })?;
        }
    }

    Ok(())
}

/// A virtual CPU that can be run until an exit occurs
pub struct VirtualCPU {}

impl VirtualCPU {
    /// Run the given hypervisor until a halt instruction is reached
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub fn run(
        hv: &mut dyn Hypervisor,
        outb_handle_fn: Arc<Mutex<dyn OutBHandlerCaller>>,
        mem_access_fn: Arc<Mutex<dyn MemAccessHandlerCaller>>,
    ) -> Result<()> {
        loop {
            if hv.get_termination_status().load() {
                #[cfg(target_os = "linux")]
                hv.set_run_cancelled(true);
                int_counter_inc!(&NumberOfCancelledGuestExecutions);
                log_then_return!(ExecutionCanceledByHost());
            }

            match hv.run()? {
                HyperlightExit::Halt() => {
                    break;
                }
                HyperlightExit::IoOut(port, data, rip, instruction_length) => {
                    hv.handle_io(port, data, rip, instruction_length, outb_handle_fn.clone())?
                }

                HyperlightExit::Mmio(addr) => {
                    mem_access_fn
                        .clone()
                        .lock()
                        .map_err(|e| new_error!("error locking: {:?}", e))?
                        .call()?;
                    log_then_return!("MMIO access address {:#x}", addr);
                }
                HyperlightExit::AccessViolation(addr, tried, region_permisson) => {
                    log_then_return!(HyperlightError::MemoryAccessViolation(
                        addr,
                        tried,
                        region_permisson
                    ));
                }
                HyperlightExit::Cancelled() => {
                    // Shutdown is returned when the host has cancelled execution
                    // TODO: we should probably make the VM unusable after this
                    #[cfg(target_os = "linux")]
                    hv.set_run_cancelled(true);
                    int_counter_inc!(&NumberOfCancelledGuestExecutions);
                    log_then_return!(ExecutionCanceledByHost());
                }
                HyperlightExit::Unknown(reason) => {
                    log_then_return!("Unexpected VM Exit {:?}", reason);
                }
                HyperlightExit::Retry() => continue,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::hypervisor::hypervisor_handler::{execute_vcpu_action, start_hypervisor_handler};
    use crate::hypervisor::hypervisor_handler::{InitArgs, VCPUAction};
    use hyperlight_testing::dummy_guest_as_string;

    use super::{
        handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper},
        Hypervisor,
    };
    use crate::sandbox::hypervisor::HypervisorWrapper;
    use crate::sandbox::SandboxConfiguration;
    use crate::{
        mem::{
            layout::SandboxMemoryLayout,
            mgr::SandboxMemoryManager,
            ptr::{GuestPtr, RawPtr},
            ptr_offset::Offset,
        },
        new_error,
        sandbox::{uninitialized::GuestBinary, UninitializedSandbox},
    };
    use crate::{sandbox::WrapperGetter, Result};
    use std::path::Path;
    use std::time::Duration;

    pub(crate) fn test_initialise<NewFn>(
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
        new_fn: NewFn,
    ) -> Result<()>
    where
        NewFn: Fn(&SandboxMemoryManager, GuestPtr, GuestPtr) -> Result<Box<dyn Hypervisor>>,
    {
        let filename = dummy_guest_as_string().map_err(|e| new_error!("{}", e))?;
        if !Path::new(&filename).exists() {
            return Err(new_error!(
                "test_initialise: file {} does not exist",
                filename
            ));
        }

        let mut sandbox =
            UninitializedSandbox::new(GuestBinary::FilePath(filename.clone()), None, None, None)?;

        let mem_mgr = sandbox.get_mgr_wrapper_mut().as_mut();
        let shared_mem = &mem_mgr.shared_mem;
        let rsp_ptr = {
            let mem_size: u64 = shared_mem.mem_size().try_into()?;
            let u64_val = mem_mgr.set_up_hypervisor_partition(mem_size)?;
            let base_addr_u64 = u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?;
            let offset = Offset::from(u64_val - base_addr_u64);
            GuestPtr::try_from(offset)
        }?;
        let pml4_ptr = {
            let offset_u64 = u64::try_from(SandboxMemoryLayout::PML4_OFFSET)?;
            let offset = Offset::from(offset_u64);
            GuestPtr::try_from(offset)
        }?;

        let hv = new_fn(mem_mgr, rsp_ptr, pml4_ptr)?;
        let hv_wrapper = HypervisorWrapper::new(
            Some(hv),
            outb_hdl.clone(),
            mem_access_hdl.clone(),
            Duration::from_millis(SandboxConfiguration::DEFAULT_MAX_EXECUTION_TIME as u64),
            #[cfg(target_os = "linux")]
            Duration::from_millis(SandboxConfiguration::DEFAULT_MAX_WAIT_FOR_CANCELLATION as u64),
        );

        // call initialise on the hypervisor implementation with specific values
        // for PEB (process environment block) address, seed and page size.
        //
        // these values are not actually used, they're just checked inside
        // the dummy guest, and if they don't match these values, the dummy
        // guest issues a write to an invalid memory address, which in turn
        // fails this test.
        //
        // in this test, we're not actually testing whether a guest can issue
        // memory operations, call functions, etc... - we're just testing
        // whether we can configure the shared memory region, load a binary
        // into it, and run the CPU to completion (e.g., a HLT interrupt)

        start_hypervisor_handler(hv_wrapper.get_hypervisor_arc()?.clone())?;

        execute_vcpu_action(
            &hv_wrapper,
            VCPUAction::Initialise(InitArgs::new(
                RawPtr::from(0x230000),
                1234567890,
                4096,
                outb_hdl,
                mem_access_hdl,
            )),
            None,
        )
    }
}
