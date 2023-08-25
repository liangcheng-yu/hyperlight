use anyhow::anyhow;
use hyperlight_host::func::exports as exported_funcs;

/// Get the stack boundary, or `0` if it can't be gotten or isn't
/// supported on this OS
#[no_mangle]
pub extern "C" fn exports_get_stack_boundary() -> u64 {
    exported_funcs::get_stack_boundary().unwrap_or(0)
}

/// Get the size of memory pages as reported by the operating system,
/// or `0` if it can't be gotten
#[no_mangle]
pub extern "C" fn exports_get_os_page_size() -> u32 {
    u32::try_from(exported_funcs::get_os_page_size()).unwrap_or(0)
}

/// A C-compatible struct to represent nanosecond-precision duration
/// measurements in an FFI-compatible manner
#[repr(C)]
#[derive(Default)]
pub struct Duration {
    /// The number of whole seconds in this duration
    pub seconds: u64,
    /// The remaining number of nanoseconds in this duration
    pub nanoseconds: u32,
}

impl From<std::time::Duration> for Duration {
    fn from(val: std::time::Duration) -> Self {
        Self {
            seconds: val.as_secs(),
            nanoseconds: val.subsec_nanos(),
        }
    }
}

/// Get the time since boot, in microseconds, or `0` if it can't
/// be gotten or isn't supported on this OS
#[no_mangle]
pub extern "C" fn exports_nanos_since_epoch() -> Duration {
    exported_funcs::get_dur_since_epoch()
        .map_err(|e| anyhow!("error getting duration since epoch: {:?}", e))
        .map(Duration::from)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::Duration;
    #[test]
    fn duration_conversion() {
        {
            // converting from a std::time::Duration with no nanos
            let val = Duration::from(std::time::Duration::new(100, 0));
            assert_eq!(0, val.nanoseconds);
            assert_eq!(100, val.seconds);
        }
        {
            // converting from a std::time::Duration with no seconds or nanos
            let val = Duration::from(std::time::Duration::new(0, 0));
            assert_eq!(0, val.nanoseconds);
            assert_eq!(0, val.seconds);
        }
    }
}
