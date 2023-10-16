use crate::Result;
use std::time::Duration;
use std::time::SystemTime;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::GetCurrentThreadStackLimits;

// TODO: this function is only required by WASM Guest, it should be moved to the Rust implementation of Hyperlight WASM
// This can only be done once we have a Rust implementation of Hyperlight and a C API for the Rust implementation of Hyperlight WASM.

/// This is required by os_thread_get_stack_boundary() in WAMR
/// (which is needed to handle AOT compiled WASM)
pub fn get_stack_boundary() -> Result<u64> {
    #[cfg(target_os = "linux")]
    {
        //TODO: This should be implemented for Linux if we re-enable in process support on Linux.
        panic!("Hyperlight only supports in process execution on Windows.");
    }
    #[cfg(target_os = "windows")]
    {
        use std::ptr::addr_of_mut;
        // This implementation mirrors the implementation in WAMR at
        // https://github.com/bytecodealliance/wasm-micro-runtime/blob/main/core/shared/platform/windows/win_thread.c#L665
        let (low, _high) = {
            let mut low = 0_usize;
            let mut high = 0_usize;
            unsafe {
                // safety: we know low and high are both memory we own
                GetCurrentThreadStackLimits(addr_of_mut!(low), addr_of_mut!(high))
            };
            (low, high)
        };
        let pg_size = get_os_page_size();
        // 4 pages are set unaccessible by system, we reserved
        // one more page at least for safety
        let val_usize = low + (pg_size * 5);
        Ok(u64::try_from(val_usize)?)
    }
}

// TODO: this function is only required by WASM Guest, it should be removed. The Rust implementation of Hyperlight WASM has its own implementation, but this can only be done once we have a C API for the Rust implementation of Hyperlight WASM.
/// Get the time since the Unix Epoch as a `Duration`.
///
/// Required by `os_time_get_boot_microsecond()` in WAMR for
/// profiling and logging.
///
/// This function may return an `Err`, since there's no simple way of
/// getting system time in a cross-platform way without encountering
/// clock skew (e.g. getting monotonic times).
///
/// While this problem is unlikely to be a problem for short-lived
/// VMs, the failure mode is nevertheless being exposed to callers.
pub fn get_dur_since_epoch() -> Result<Duration> {
    Ok(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?)
}

/// Get the page size for the operating system
// TODO: once the C API for the Hyperlight is done this function should be removed.
pub fn get_os_page_size() -> usize {
    page_size::get()
}
