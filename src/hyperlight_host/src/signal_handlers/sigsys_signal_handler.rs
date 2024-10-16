use std::cell::RefCell;
use std::ffi::c_void;

use libc::{c_int, sigaction, siginfo_t, ucontext_t, SIGSYS};
use once_cell::sync::OnceCell;

use crate::signal_handlers::{
    delegate_to_old_handler, register_signal_handler, IS_HYPERLIGHT_THREAD,
};

// Store the old SIGSYS handler using OnceCell for thread-safe initialization
static OLD_SIGSYS_HANDLER: OnceCell<sigaction> = OnceCell::new();

// Thread-local storage for syscall information
thread_local! {
    pub static SYSCALL_NUMBER: RefCell<Option<usize>> = const { RefCell::new(None) };
    pub static IOCTL_PARAM: RefCell<Option<usize>> = const { RefCell::new(None) };
}

/// Registers the SIGSYS signal handler once per process.
pub fn register_signal_handler_once() -> crate::Result<()> {
    register_signal_handler(SIGSYS, handle_signal, &OLD_SIGSYS_HANDLER)
}

/// The custom SIGSYS signal handler.
extern "C" fn handle_signal(signal: c_int, info: *mut siginfo_t, context: *mut c_void) {
    if signal != SIGSYS {
        // Unexpected signal; ignore
        return;
    }

    // Check if the current thread is a hyperlight thread
    IS_HYPERLIGHT_THREAD.with(|is_hyperlight_thread| {
        if *is_hyperlight_thread.borrow() {
            // Handle the signal for hyperlight threads
            const SI_OFF_SYSCALL: isize = 6;

            // SAFETY: Assuming 'info' points to a valid 'siginfo_t'
            let syscall = unsafe { *(info as *const i32).offset(SI_OFF_SYSCALL) as usize };

            eprintln!("Disallowed syscall: {}", syscall);
            SYSCALL_NUMBER.with(|syscall_num| {
                *syscall_num.borrow_mut() = Some(syscall);
            });

            if syscall == libc::SYS_ioctl as usize {
                let ucontext = unsafe { &*(context as *const ucontext_t) };
                let mcontext = &ucontext.uc_mcontext;
                let ioctl_param = mcontext.gregs[9];

                eprintln!("Disallowed ioctl: {:x}", ioctl_param);
                IOCTL_PARAM.with(|param| {
                    *param.borrow_mut() = Some(ioctl_param as usize);
                });
            }
        } else {
            // Not a hyperlight thread; delegate to the old handler or default
            delegate_to_old_handler(signal, info, context, &OLD_SIGSYS_HANDLER);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::{mem, thread};

    use libc::{c_int, sigaction, sigemptyset, siginfo_t, SA_SIGINFO, SIGSYS};
    use once_cell::sync::OnceCell;
    use serial_test::serial;

    use super::*;
    use crate::signal_handlers::mark_as_hyperlight_thread;

    // Mock old handler flag
    static OLD_HANDLER_CALLED: OnceCell<Arc<AtomicBool>> = OnceCell::new();

    /// Mock old SIGSYS handler that sets a flag when called
    extern "C" fn mock_old_handler(_signal: c_int, _info: *mut siginfo_t, _context: *mut c_void) {
        if let Some(flag) = OLD_HANDLER_CALLED.get() {
            eprintln!("Called mock_old_handler");
            flag.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    #[serial]
    fn test_signal_delegation_to_old_handler() -> anyhow::Result<()> {
        // Initialize the flag
        let old_handler_flag = Arc::new(AtomicBool::new(false));
        OLD_HANDLER_CALLED.set(old_handler_flag.clone()).unwrap();

        // Step 1: Register the mock old handler
        unsafe {
            let mut old_action: sigaction = mem::zeroed();

            // Define the old signal handler
            let mut old_sigaction: sigaction = mem::zeroed();
            old_sigaction.sa_sigaction = mock_old_handler as usize;
            old_sigaction.sa_flags = SA_SIGINFO;
            sigemptyset(&mut old_sigaction.sa_mask);

            // Register the old handler and save the previous one
            if sigaction(SIGSYS, &old_sigaction, &mut old_action) != 0 {
                panic!("Failed to register mock old SIGSYS handler");
            }
        }

        // Step 2: Register the custom SIGSYS handler
        register_signal_handler_once().unwrap();

        // Step 3: Spawn a non-hyperlight thread that triggers SIGSYS
        let non_hyperlight_thread = thread::spawn(move || {
            // This thread is NOT marked as a hyperlight thread
            // Directly trigger SIGSYS
            unsafe {
                libc::raise(SIGSYS);
            }
        });

        // Wait for the non-hyperlight thread to finish
        non_hyperlight_thread
            .join()
            .expect("Failed to join non-app thread");

        // Step 4: Verify that the mock old handler was called
        assert!(
            old_handler_flag.load(Ordering::SeqCst),
            "Old SIGSYS handler was not called from non-app thread"
        );

        // Step 5: Spawn a hyperlight thread that triggers SIGSYS
        let hyperlight_thread = thread::spawn(move || {
            // Mark this thread as a hyperlight thread
            mark_as_hyperlight_thread();

            // Trigger SIGSYS, which should be handled by the custom handler
            unsafe {
                libc::raise(SIGSYS);
            }
        });

        // Wait for the hyperlight thread to finish
        hyperlight_thread.join().expect("Failed to join app thread");

        Ok(())
    }

    #[test]
    #[serial]
    fn test_signal_handling_in_hyperlight_thread() -> anyhow::Result<()> {
        // Step 1: Register the custom SIGSYS handler
        register_signal_handler_once().unwrap();

        // Step 2: Spawn a hyperlight thread that triggers SIGSYS
        let hyperlight_thread = thread::spawn(move || {
            // Mark this thread as a hyperlight thread
            mark_as_hyperlight_thread();

            // Trigger SIGSYS, which should be handled by the custom handler
            unsafe {
                libc::raise(SIGSYS);
            }
        });

        // Wait for the hyperlight thread to finish
        hyperlight_thread.join().expect("Failed to join app thread");

        Ok(())
    }
}
