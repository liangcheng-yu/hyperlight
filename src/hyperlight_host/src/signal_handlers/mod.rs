#[cfg(feature = "seccomp")]
pub mod sigsys_signal_handler;

pub mod hltimeout_signal_handler;

use std::mem;

use libc::{c_int, c_void, sigaction, sigemptyset, siginfo_t, SA_SIGINFO};
use once_cell::sync::OnceCell;

use crate::new_error;

// Thread-local storage to identify hyperlight threads
thread_local! {
    pub static IS_HYPERLIGHT_THREAD: std::cell::RefCell<bool> = const { std::cell::RefCell::new(false) };
}

/// Marks the current thread as a hyperlight thread.
pub(crate) fn mark_as_hyperlight_thread() {
    IS_HYPERLIGHT_THREAD.with(|is_hyperlight_thread| {
        *is_hyperlight_thread.borrow_mut() = true;
    });
}

// A helper function to register a signal handler
pub(crate) fn register_signal_handler(
    signal: c_int,
    handler_fn: extern "C" fn(c_int, *mut siginfo_t, *mut c_void),
    old_handler: &OnceCell<sigaction>,
) -> crate::Result<()> {
    old_handler.get_or_init(|| unsafe {
        let mut old_action: sigaction = mem::zeroed();
        let mut new_action: sigaction = mem::zeroed();

        new_action.sa_sigaction = handler_fn as usize;
        new_action.sa_flags = SA_SIGINFO;
        sigemptyset(&mut new_action.sa_mask);

        if sigaction(signal, &new_action, &mut old_action) != 0 {
            new_error!("Failed to register signal handler for signal {}", signal);
        }

        old_action
    });

    Ok(())
}

// A helper function to delegate to the old handler
pub(crate) fn delegate_to_old_handler(
    signal: c_int,
    info: *mut siginfo_t,
    context: *mut c_void,
    old_handler: &OnceCell<sigaction>,
) {
    unsafe {
        if let Some(old_action) = old_handler.get() {
            // Check if the old handler has a sa_sigaction
            if old_action.sa_sigaction != 0 {
                let old_handler_fn: extern "C" fn(c_int, *mut siginfo_t, *mut c_void) =
                    std::mem::transmute(old_action.sa_sigaction);
                // Delegate to the old handler
                old_handler_fn(signal, info, context);
            } else {
                // No old handler; set default and re-raise the signal
                libc::signal(signal, libc::SIG_DFL);
                libc::raise(signal);
            }
        } else {
            // No previous handler; set default and re-raise the signal
            libc::signal(signal, libc::SIG_DFL);
            libc::raise(signal);
        }
    }
}
