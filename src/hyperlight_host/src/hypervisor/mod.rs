#[cfg(target_os = "linux")]
use crate::error::HyperlightError::HostFailedToCancelGuestExecutionSendingSignals;
use crate::error::HyperlightError::{ExecutionCanceledByHost, HostFailedToCancelGuestExecution};
use crate::log_then_return;
use crate::new_error;
use crate::Result;
use crossbeam::atomic::AtomicCell;
#[cfg(target_os = "linux")]
use libc::{c_void, pthread_kill, pthread_self, siginfo_t, ESRCH};
use log::error;
#[cfg(target_os = "linux")]
use log::info;
#[cfg(target_os = "linux")]
use vmm_sys_util::signal::{register_signal_handler, SIGRTMIN};
#[cfg(target_os = "windows")]
use windows::Win32::System::Hypervisor::WHvCancelRunVirtualProcessor;
#[cfg(target_os = "windows")]
use windows::Win32::System::Hypervisor::WHV_PARTITION_HANDLE;
/// Handlers for Hypervisor custom logic
pub mod handlers;
/// HyperV-on-linux functionality
#[cfg(target_os = "linux")]
pub mod hyperv_linux;
#[cfg(target_os = "windows")]
/// Hyperv-on-windows functionality
pub(crate) mod hyperv_windows;
#[cfg(target_os = "linux")]
/// Hypervisor-generic memory utilities
pub mod hypervisor_mem;
#[cfg(target_os = "linux")]
/// Functionality to manipulate KVM-based virtual machines
pub mod kvm;
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
#[cfg(target_os = "windows")]
use crate::hypervisor::hyperv_windows::HypervWindowsDriver;

use self::handlers::{
    MemAccessHandlerCaller, MemAccessHandlerWrapper, OutBHandlerCaller, OutBHandlerWrapper,
};
use crate::mem::ptr::RawPtr;
use std::{
    any::Any,
    cell::RefCell,
    fmt::Debug,
    sync::{Arc, Condvar, Mutex},
    thread,
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
    /// The vCPU execution has been cancelled
    Cancelled(),
    /// The vCPU has exited for a reason that is not handled by Hyperlight
    Unknown(String),
    /// The operation should be retried, for example this can happen on Linux where a call to run the CPU can return EAGAIN
    Retry(),
}

// These two thread locals are used to store data for the spawned thread running the vCPU.

thread_local!(static CANCEL_RUN_REQUESTED: RefCell<Arc<AtomicCell<bool>>> = RefCell::new(Arc::new(AtomicCell::new(false))));
#[cfg(target_os = "linux")]
thread_local!(static RUN_CANCELLED: RefCell<Arc<AtomicCell<bool>>> = RefCell::new(Arc::new(AtomicCell::new(false))));

/// A common set of hypervisor functionality
pub trait Hypervisor: Debug + Sync + Send {
    /// get a mutable trait object from self
    fn as_mut_hypervisor(&mut self) -> &mut dyn Hypervisor;

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
        max_execution_time: Duration,
        max_wait_for_cancellation: Duration,
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

    /// Run the internally stored vCPU until a HLT instruction.
    fn execute_until_halt(
        &mut self,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
        timeout: Duration,
        timeout_wait_to_cancel: Duration,
    ) -> Result<()> {
        // We are going to run all interactions with the VM on a separate thread
        // This enables us to cancel the execution of the VM from the main thread if it exceeds a configurable timeout
        // It also means that (in Linux at least) we can limit the capabilities of the thread that is running the VM
        // by using seccomp filters to limit the syscalls that the thread can make.
        // This is important as we are running untrusted code in the VM and we want to limit what it can do.

        // We need to use a thread local to track if we are running on the spawned thread or not
        // If a call is made to a guest which in turn calls the host and then the host calls the guest again we will end up re-entering
        // this function, in this case we dont want to spawn a new thread as we are already running on a spawned thread.
        // In the spawned thread we will set the value os SPAWNED_THREAD to true and then here we check it before spawning a new thread.
        // This means that by checking the value of the thread local we can determine if we are running on the spawned thread or not.

        // if we are on the spawned thread call the run method directly and then return

        thread_local!(static SPAWNED_THREAD: RefCell<bool> = RefCell::new(false));

        if SPAWNED_THREAD.with(|st| *st.borrow()) {
            return VirtualCPU::run(self.as_mut_hypervisor(), outb_handle_fn, mem_access_fn);
        }

        // When we need to cancel the execution there are 2 possible cases we have to deal with depending on
        // if the vCPU is currently running or not.
        //
        // 1. If the VCPU is executing then we need to cancel the execution
        // 2. If the VCPU is not executing then we need to signal to the thread that it should exit the loop.
        //
        // For the first case in Linux we send a signal to the thread running the vCPU to interrupt it and cause an EINTR error
        // on the underlying VM run call.
        //
        // For the second case we set a flag that is checked on each iteration of the run loop and if it is set to true then the loop
        // will exit.
        // We will use the following AtomicCell to communicate between the main thread and the spawned thread running the vCPU.
        // We will create this on the main thread the first time we enter this function and store it TLS and
        // then create a clone which we will move to the spawned thread which will then place it in its TLS so that it
        // is accessible in the run loop if we re-enter the function

        let cancel_run_requested = CANCEL_RUN_REQUESTED.with(|cr| cr.borrow().clone());
        let cancel_run_requested_clone = cancel_run_requested.clone();

        // On Linux we have another problem to deal with. The way we terminate a running vCPU  (case 1 above)
        // is to send a signal to the thread running the vCPU to interrupt it.
        //
        // There is a possibility that the signal is sent and received just before the thread calls run on the vCPU,
        // (between the check on the cancelled_run variable and the call to run) - see this SO question for more details on why this is needed
        // https://stackoverflow.com/questions/25799667/fixing-race-condition-when-sending-signal-to-interrupt-system-call)
        //
        // To solve this we need to keep sending the signal until we know that the spawned thread thread knows it should cancel the execution.
        // To do this we will use another atomic cell and arc to communicate between the main thread and the spawned thread running the VCPU.
        // This variable will be set when the thread has received the instruction to cancel the execution and will be checked
        // in the code which sends the signal to terminate to know there is no longer a needed to send a signal.
        // Again we will create this on the main thread the first time we enter this function and store it TLS and
        // then create a clone which we will move to the spawned thread which will then place it in its TLS so that it
        // is accessible in the run loop if we re-enter the function

        #[cfg(target_os = "linux")]
        let run_cancelled = {
            let run_cancelled = RUN_CANCELLED.with(|rc| rc.borrow().clone());
            let run_cancelled_clone = run_cancelled.clone();
            (run_cancelled, run_cancelled_clone)
        };

        // We need to use a thread scope to run the spwaned thread as we need to allow non static lifetime data
        // to be passed to the thread routine.
        //
        // On Linux to stop the exection we need to signal the thread to cause an EINTR error on the underlyting VM run call
        // the EINTR error is then handled by the run method and it returns a HyperlightExit::Cancelled to indicate that the execution was cancelled.
        //
        // The vmm_sys_util crate provides a Killable trait that makes it easy to signal a thread which we would have used to signal the thread,
        // however it is not implemented for ScopedJoinHandle which is the type of the JoinHandle returned by thread::scope::spawn and
        // its also not possible to easily imlplement it for ScopedJoinHandle as ScopedJoinHandle does not expose the native thread details
        // (JoinHandle does this as it exposes an as_inner() method that returns a libc Thread struct whereas
        // ScopedJoinHandle only exposes the Rust Thread struct which is no good for signalling the thread).
        //
        // So the way we are going to solve this for now is to get the p_thread_t via a libc call in the spawned thread and then use an atomicell
        // to make it available to host thread, then if we need to signal the thread to cancel the execution we can get the thread_id
        // from the AtomicCell and call pthread_kill on it.

        #[cfg(target_os = "linux")]
        let thread_details = {
            let spawned_thread_id = Arc::new(AtomicCell::new(u64::MAX));
            let spawned_thread_id_clone = spawned_thread_id.clone();
            (spawned_thread_id, spawned_thread_id_clone)
        };

        // On Windows we have a slightly different problem, Windows supports an API that allows us to cancel the execution of a vCPU but
        // to use that from here would require us to expose an API from the WindowsHypervisor imlementation, since in the thread spawned below
        // we move the mutable self reference into the thread closure it makes it painful to be able to call such a cancel function from the main thread (since we would need
        // the mutable reference to call run we would not alos be able to have a reference to call cancel) As Windows is happy for us to interact with the VM from different threads
        // the solution we are going to use is to get the partition handle from the WindowsHypervisor implementation
        // and then use that handle in a call to the cancel function from the main thread. We only need the partition handle to call the cancel function as we are always
        // ever creating a single vCPU in a partition and therefore can default the other parameters to 0.

        #[cfg(target_os = "windows")]
        let partition_handle = {
            // To get the partition handle we need to downcast the Hypervisor trait object to a WindowsHypervisorDriver
            let hyperv_windows_driver: &HypervWindowsDriver =
                match self.as_any().downcast_ref::<HypervWindowsDriver>() {
                    Some(b) => b,
                    None => {
                        log_then_return!("Expected a WindowsHypervisorDriver");
                    }
                };
            hyperv_windows_driver.get_partition_hdl()
        };

        // We use a condvar to notify the main thread when the spawned thread has completed
        // We can wait on this with a timeout in the main thread to determine if the spawned thread has completed
        // or the execution needs to be cancelled.

        let cv_pair1 = Arc::new((Mutex::new(false), Condvar::new()));
        let cv_pair2 = Arc::clone(&cv_pair1);

        // ** NOTE** //
        // any errors returned from the thread scope will cause an implicit join on the spawned thread
        // if the spawned thread has not been joined, this may cause the main thread to hang.
        // There should be no cases where this happens apart from the 2 explicit ones below where thread cancellation fails

        thread::scope(|s| {
            // Using builder spawn scoped so that we receive an error if the thread creation fails, otherwise we would just panic.

            let join_handle = thread::Builder::new()
                // TODO: We should create a unique id for the Sandbox and use it as part of the thread name.
                .name("Hyperlight vCPU".to_string())
                .spawn_scoped(s, move || -> Result<()> {
                    // If any error occurs we need to make sure that all errors are handle explicitly and paced in the result variable
                    // so that we exit the function and set the cond_var to notify the main thread that we have completed.
                    // Therefore we cannot use the ? operator here as that will cause us to return from the thread closure and not set the cond_var.
                    // We also cannot use bail! or return Err() as that will cause the same problem.
                    // Not setting the condvar would mean that we relied on timing out the wait for the thread to complete when an error occurs.

                    let mut result = Ok(());

                    SPAWNED_THREAD.with(|st| {
                        *st.borrow_mut() = true;
                    });

                    // Store the passed cancel_run_requested in TLS so that it is accessible in the run loop
                    // if we re-enter the function
                    CANCEL_RUN_REQUESTED.with(|crc| {
                        *crc.borrow_mut() = cancel_run_requested.clone();
                    });

                    #[cfg(target_os = "linux")]
                    {
                        // As we cannot use the Killable trait we need to get the pthread_t via a libc call

                        let thread_id = unsafe { pthread_self() };
                        thread_details.1.store(thread_id);

                        // Register a signal handler to cancel the execution of the VCPU on Linux
                        // On Windows we dont need to do anything as we can just call the cancel function.

                        extern "C" fn handle_signal(_: i32, _: *mut siginfo_t, _: *mut c_void) {}
                        match register_signal_handler(SIGRTMIN(), handle_signal) {
                            Ok(_) => {}
                            Err(e) => {
                                result =
                                    Err(new_error!("failed to register signal handler: {:?}", e));
                            }
                        }

                        // Store the passed run_cancelled in TLS so that it is accessible in the run loop
                        // if we re-enter the function

                        RUN_CANCELLED.with(|rc| {
                            *rc.borrow_mut() = run_cancelled.1.clone();
                        });
                    }

                    if result.is_ok() {
                        result = VirtualCPU::run(
                            self.as_mut_hypervisor(),
                            outb_handle_fn,
                            mem_access_fn,
                        );
                    }

                    let (lock, cvar) = &*cv_pair2;
                    match lock.lock() {
                        Ok(mut done) => {
                            *done = true;
                        }
                        Err(e) => {
                            result = Err(new_error!("Error Locking {:?}", e));
                        }
                    }
                    // Even if the lock fails we should stll try and notify the main thread that we have completed
                    // although the fact that we fail here means in theory that the main thread has panicked while holding the lock
                    cvar.notify_one();
                    result
                })?;

            let (lock, cvar) = &*cv_pair1;

            // The following is inside a code block to ensure that the lock is released before we call join on the thread, otherwise we may end up deadlocking.
            // This means that the spawned thread can acquire the lock and notify the cond_var after we return from the wait_timeout.
            // Later on we will acquire the lock again and check if the thread has completed or if the wait timed out.

            let timed_out_result = {
                match lock.lock() {
                    Ok(done) => {
                        // TODO: This might not be a good enough implementation, see the docs for comments about wait_timeout, it may not wait for the full duration
                        // We could mitigate this by using a loop and checking the time elapsed and then waiting for the remaining time.

                        // FYI the docs say that the wait timeout will unlock the mutex while waiting and then relock it when it returns.
                        // This is why we dont get a deadlock with the thread that is running the VCPU.

                        // Its fine to return an error here as this means that the spawned thread has panicked while holding the lock
                        // so we wont hang.
                        match cvar.wait_timeout(done, timeout) {
                            Ok(result) => Ok(result.1.timed_out()),
                            Err(e) => Err(new_error!("Failed to wait for thread {:?}", e)),
                        }
                    }
                    // Its fine to return an error from locking here as this would mean that the spawned thread has panicked while holding the lock
                    // so we wont hang.
                    Err(e) => Err(new_error!("Error Locking {:?}", e)),
                }
            };

            // If the thread completed return the result, otherwise check again to see if it completed in the meantime,
            // if it has not, try to cancel the execution then wait for the result and then return it.

            let thread_execution_result = match timed_out_result {
                Ok(timed_out) => match timed_out {
                    // The spawned thread completed
                    false => match join_handle.join() {
                        Ok(result) => result,
                        // Its fine to return error here as we know that the spawned thread has completed
                        Err(e) => {
                            log_then_return!(new_error!("Join thread returned an error {:?}", e));
                        }
                    },
                    // The wait timed out the spawned thread may not have completed
                    true => {
                        // Its fine to return an error from locking here as this would mean that the spawned thread has panicked while holding the lock
                        // so we wont hang.
                        let done = lock
                            .lock()
                            .map_err(|e| new_error!("Error Locking {:?}", e))?;

                        // If the run vcpu did not complete between the time we returned from the wait_timeout
                        // and the time we acquired the lock then we need to cancel the execution.
                        // If it is completed then we dont need to do anything, the cond_var will be notified.
                        // The wait_timeout_while below will return immediately
                        let terminate_execution = match *done {
                            true => Ok(()),
                            false => terminate_execution(
                                timeout,
                                cancel_run_requested_clone,
                                #[cfg(target_os = "linux")]
                                timeout_wait_to_cancel,
                                #[cfg(target_os = "linux")]
                                thread_details.0.clone(),
                                #[cfg(target_os = "linux")]
                                run_cancelled.0.clone(),
                                #[cfg(target_os = "windows")]
                                partition_handle,
                            ),
                        };

                        // if we successfully terminated execution then we need to wait for the thread to complete
                        // otherwise we can need to handle the error
                        // we will do this by checking to see if the spawned thread has completed and if not we will kill it.

                        match terminate_execution {
                            Ok(_) => {
                                // here we are only going to wait a maximum of timeout_wait_to_cancel for the thread to finish,
                                // if it doesn't finish in that time we will return an error
                                // if it has already completed then the wait will return immediately

                                // NOTE: the guard will be unlocked while we wait and then relocked when we return, this will enable the thread loop to notify if it finishes.
                                let result = match cvar.wait_timeout_while(
                                    done,
                                    timeout_wait_to_cancel,
                                    |&mut done| !done,
                                ) {
                                    Ok(result) => result,
                                    Err(e) => {
                                        // This should only happen if the spawned thread has panicked while holding the lock
                                        // so its safe to bail here as we know that the spawned thread has completed.
                                        log_then_return!(new_error!(
                                            "Failed to wait for thread {:?}",
                                            e
                                        ));
                                    }
                                };

                                match *result.0 {
                                    // Spawned thread is finished.
                                    true => match join_handle.join() {
                                        Ok(result) => result,
                                        Err(e) => {
                                            log_then_return!(new_error!(
                                                "Failed to join thread {:?}",
                                                e
                                            ));
                                        }
                                    },
                                    false => {
                                        // The thread still has not signalled its complete
                                        // This means that it is potentially hung calling a host function.
                                        // check if it has completed return an error specifying that we failed to cancel the execution.
                                        // NOTE that this will hang the calling thread until the spawned thread completes.
                                        // this can happen if the guest has called a host function that has hung, is slow or never returns.
                                        // TODO: see https://github.com/deislabs/hyperlight/issues/951
                                        match join_handle.is_finished() {
                                            true => Ok(()),
                                            false => {
                                                log_then_return!(HostFailedToCancelGuestExecution());
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                // We failed to terminate the execution, we need
                                // check if it has completed and if not we will return an error.
                                // NOTE that this will hang the calling thread until the spawned thread completes.
                                // this can happen if the guest has called a host function that has hung, is slow or never returns.
                                // TODO: see https://github.com/deislabs/hyperlight/issues/951
                                match join_handle.is_finished() {
                                    true => Ok(()),
                                    false => {
                                        log_then_return!(e);
                                    }
                                }
                            }
                        }
                    }
                },
                // Failed to get the lock, this means that the scoped thread has panicked while holding the lock
                // we can bail here as we know that the thread has exited and the scope will join it implicitly.
                Err(e) => {
                    log_then_return!("Failed to wait for thread {:?}", e);
                }
            };
            thread_execution_result
        })
    }

    /// Run the vCPU
    fn run(&mut self) -> Result<HyperlightExit>;

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
        max_execution_time: Duration,
        max_wait_for_cancellation: Duration,
    ) -> Result<()>;

    /// Reset the stack pointer on the internal virtual CPU
    fn reset_rsp(&mut self, rsp: u64) -> Result<()>;

    /// Get the value of the stack pointer (RSP register) when this
    /// `Hypervisor` was first created
    fn orig_rsp(&self) -> Result<u64>;

    /// Allow the hypervisor to be downcast
    fn as_any(&self) -> &dyn Any;
}

fn terminate_execution(
    timeout: Duration,
    cancel_run_requested: Arc<AtomicCell<bool>>,
    #[cfg(target_os = "linux")] timeout_wait_to_cancel: Duration,
    #[cfg(target_os = "linux")] thread_details: Arc<AtomicCell<u64>>,
    #[cfg(target_os = "linux")] run_cancelled: Arc<AtomicCell<bool>>,
    #[cfg(target_os = "windows")] partition_handle: WHV_PARTITION_HANDLE,
) -> Result<()> {
    error!(
        "Execution timed out after {} milliseconds , cancelling execution",
        timeout.as_millis()
    );
    cancel_run_requested.store(true);
    #[cfg(target_os = "linux")]
    {
        let thread_id = thread_details.load();
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
            thread::sleep(Duration::from_micros(500));
        }
        if !run_cancelled.load() {
            log_then_return!(HostFailedToCancelGuestExecutionSendingSignals(count));
        }
    }
    #[cfg(target_os = "windows")]
    {
        unsafe {
            // Its intentional that we panic on an error here, if we bail then we could hang forever
            WHvCancelRunVirtualProcessor(partition_handle, 0, 0)
                .map_err(|e| new_error!("Failed to cancel guest execution {:?}", e))?;
        };
    }
    Ok(())
}

struct VirtualCPU {}

impl VirtualCPU {
    fn run<'a>(
        hv: &mut dyn Hypervisor,
        outb_handle_fn: Arc<Mutex<dyn OutBHandlerCaller + 'a>>,
        mem_access_fn: Arc<Mutex<dyn MemAccessHandlerCaller + 'a>>,
    ) -> Result<()> {
        let cancel_run_requested = CANCEL_RUN_REQUESTED.with(|cr| cr.borrow().clone());

        #[cfg(target_os = "linux")]
        let run_cancelled = RUN_CANCELLED.with(|rc| rc.borrow().clone());

        let result = Ok(());

        loop {
            if cancel_run_requested.load() {
                #[cfg(target_os = "linux")]
                run_cancelled.store(true);
                log_then_return!(ExecutionCanceledByHost());
            }

            match hv.run()? {
                HyperlightExit::Halt() => break,
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
                HyperlightExit::Cancelled() => {
                    // Shutdown is returned when the host has cancelled execution
                    // TODO: we should probably make the VM unusable after this
                    #[cfg(target_os = "linux")]
                    run_cancelled.store(true);
                    log_then_return!(ExecutionCanceledByHost());
                }
                HyperlightExit::Unknown(reason) => {
                    log_then_return!("Unexpected VM Exit {:?}", reason);
                }
                HyperlightExit::Retry() => continue,
            }
        }
        result
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper},
        Hypervisor,
    };
    use crate::Result;
    use crate::{
        mem::{
            layout::SandboxMemoryLayout,
            mgr::SandboxMemoryManager,
            ptr::{GuestPtr, RawPtr},
            ptr_offset::Offset,
        },
        new_error,
        sandbox::{mem_mgr::MemMgrWrapperGetter, uninitialized::GuestBinary, UninitializedSandbox},
        testing::dummy_guest_path,
    };
    use std::{path::Path, time::Duration};

    pub(crate) fn test_initialise<NewFn>(
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
        new_fn: NewFn,
    ) -> Result<()>
    where
        NewFn: Fn(&SandboxMemoryManager, GuestPtr, GuestPtr) -> Result<Box<dyn Hypervisor>>,
    {
        let filename = dummy_guest_path()?;
        if !Path::new(&filename).exists() {
            return Err(new_error!(
                "test_initialise: file {} does not exist",
                filename
            ));
        }

        let mut sandbox =
            UninitializedSandbox::new(GuestBinary::FilePath(filename.clone()), None, None, None)?;
        let mem_mgr = {
            let wrapper = sandbox.get_mem_mgr_wrapper_mut();
            wrapper.as_mut()
        };
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
        let mut hypervisor_impl = new_fn(mem_mgr, rsp_ptr, pml4_ptr)?;

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
        hypervisor_impl
            .initialise(
                RawPtr::from(0x230000),
                1234567890,
                4096,
                outb_hdl,
                mem_access_hdl,
                Duration::from_millis(1000),
                Duration::from_millis(10),
            )
            .map_err(|e| new_error!("Error running hypervisor against {} ({:?})", filename, e))
    }
}
