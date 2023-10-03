#[cfg(target_os = "windows")]
use crate::hypervisor::handlers::OutBHandlerCaller;
#[cfg(target_os = "windows")]
use crate::mem::mgr::SandboxMemoryManager;
#[cfg(target_os = "windows")]
use std::sync::{Arc, Mutex};

/// When executing in-process (only Windows only), we leak the outb handler
/// wrapper so we can write a pointer to it to shared memory. The in-process
/// execution in turn follows that pointer so it can call the outb method.
///
/// Why, exactly, we need to leak the handler's memory is not important
/// for purposes of this explanation (but deals with Rust's ownership), but we
/// need to ultimately ensure we clean up the leaked memory if needed. Since
/// we know we'll never need this particular method again after the sandbox
/// is dropped, we know we can clean up its memory at that point.
///
/// This this `drop_impl` method should only be called within the `Drop`
/// implementation of a `SingleUseSandbox` or `MultiUseSandbox`, and on
/// Windows builds only.
#[cfg(target_os = "windows")]
pub(super) fn drop_impl(mgr: &SandboxMemoryManager) {
    let run_from_proc_mem = mgr.run_from_process_memory;
    if run_from_proc_mem {
        if let Ok(ctx) = mgr.get_outb_context() {
            if ctx != 0 {
                let _outb_handlercaller: Box<Arc<Mutex<dyn OutBHandlerCaller>>> =
                    unsafe { Box::from_raw(ctx as *mut Arc<Mutex<dyn OutBHandlerCaller>>) };
            }
        }
    }
}
