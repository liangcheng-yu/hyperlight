use anyhow::Result;
use std::time::SystemTime;
use std::time::{Duration, SystemTimeError};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::GetCurrentThreadStackLimits;

/// This is required by os_thread_get_stack_boundary() in WAMR
/// (which is needed to handle AOT compiled WASM)
pub(crate) fn get_stack_boundary() -> Result<u64> {
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
pub(crate) fn get_dur_since_epoch() -> Result<Duration, SystemTimeError> {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
}

pub(crate) fn get_os_page_size() -> usize {
    page_size::get()
}
