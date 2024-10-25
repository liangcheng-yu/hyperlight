/*
Copyright 2024 The Hyperlight Authors.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

use std::ffi::c_void;

use libc::{sigaction, siginfo_t, SIGRTMIN};
use once_cell::sync::OnceCell;

use crate::signal_handlers::{
    delegate_to_old_handler, register_signal_handler, IS_HYPERLIGHT_THREAD,
};

// Store the old HLTIMEOUT handler
static OLD_HLTIMEOUT_HANDLER: OnceCell<sigaction> = OnceCell::new();

/// Registers the HLTIMEOUT signal handler once per process.
pub fn register_signal_handler_once() -> crate::Result<()> {
    register_signal_handler(SIGRTMIN(), handle_signal, &OLD_HLTIMEOUT_HANDLER)
}

/// The custom HLTIMEOUT signal handler
extern "C" fn handle_signal(signal: i32, info: *mut siginfo_t, context: *mut c_void) {
    if signal != SIGRTMIN() {
        // Unexpected signal; ignore
        return;
    }

    // Check if the current thread is a hyperlight thread
    IS_HYPERLIGHT_THREAD.with(|is_hyperlight_thread| {
        if *is_hyperlight_thread.borrow() {
            eprintln!("Hyperlight thread received HLTIMEOUT signal.");
        } else {
            // Not a hyperlight thread; delegate to the old handler or default
            delegate_to_old_handler(signal, info, context, &OLD_HLTIMEOUT_HANDLER);
        }
    });
}
