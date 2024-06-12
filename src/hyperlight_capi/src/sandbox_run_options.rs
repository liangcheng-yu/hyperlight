use bitflags::bitflags;
use hyperlight_host::{log_then_return, Result};
use hyperlight_host::{HyperlightError, SandboxRunOptions as CoreSandboxRunOptions};

bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone)]
    /// Options for running a sandbox
    pub struct SandboxRunOptions: u32 {
        /// Run in a Hypervisor
        const RUN_IN_HYPERVISOR = 0b00000001;
        /// Run in process.
        ///
        /// Only available on Windows.
        const RUN_IN_PROCESS = 0b00000010;
        /// Recycle the sandbox after running
        const RECYCLE_AFTER_RUN = 0b00000100;
        /// Run from guest binary using the Windows LoadLibrary API.
        ///
        /// Only available on Windows, and if `RUN_IN_PROCESS` is also set
        const RUN_FROM_GUEST_BINARY = 0b00001000;
    }
}

/// Convert a `SandboxRunOptions` to a `hyperlight_host::SandboxRunOptions`
/// according to the flags set on the `SandboxRunOptions`.
///
/// Note that there's no `CoreSandboxRunOptions` option to represent
/// `RECYCLE_AFTER_RUN` because the semantic equivalent to `RECYCLE_AFTER_RUN`
/// is represented in the type system with `SingleUseSandbox` and
/// `MultiUseSandbox`, rather than with runtime configuration.
///
/// Therefore, `RECYCLE_AFTER_RUN` is considered in the `try_from` here
/// only to ensure it doesn't conflict with other config.
///
/// Finally, since the `value` parameter -- a `SandboxRunOptions` type --
/// is represented as a `u32`, only valid values and the `0` value is
/// considered. `0` values will be considered the same as a
/// `CoreSandboxRunOptions::RunInHypervisor` type.
impl TryFrom<SandboxRunOptions> for CoreSandboxRunOptions {
    type Error = HyperlightError;
    fn try_from(value: SandboxRunOptions) -> Result<Self> {
        if value.is_empty() {
            // empty value is the same as the `u32` value of `0`, so
            // consider this a `RunInHypervisor`
            Ok(CoreSandboxRunOptions::RunInHypervisor)
        } else if value.contains(SandboxRunOptions::RUN_IN_PROCESS) {
            if value.contains(SandboxRunOptions::RUN_IN_HYPERVISOR) {
                log_then_return!("RUN_IN_PROCESS can't be configured with RUN_IN_HYPERVISOR");
            }

            #[cfg(target_os = "linux")]
            {
                log_then_return!("In-process mode is currently only available on Windows");
            }
            #[cfg(target_os = "windows")]
            {
                Ok(CoreSandboxRunOptions::RunInProcess(
                    value.contains(SandboxRunOptions::RUN_FROM_GUEST_BINARY),
                ))
            }
        } else if value.contains(SandboxRunOptions::RUN_IN_HYPERVISOR) {
            if value.contains(SandboxRunOptions::RUN_IN_PROCESS) {
                log_then_return!("RUN_IN_HYPERVISOR can't be configured with RUN_IN_PROCESS");
            } else if value.contains(SandboxRunOptions::RUN_FROM_GUEST_BINARY) {
                log_then_return!("RUN_IN_HYPERVISOR can't be configured with RUN_FROM_GUEST_BINARY")
            }

            Ok(CoreSandboxRunOptions::RunInHypervisor)
        } else if value.contains(SandboxRunOptions::RECYCLE_AFTER_RUN) {
            // to remain compatible with a large number of C# tests, we are
            // allowing callers to send just the RECYCLE_AFTER_RUN flag. if
            // they do so, we'll assume this implies in-hypervisor mode with
            // recycling on
            Ok(CoreSandboxRunOptions::RunInHypervisor)
        } else if value.contains(SandboxRunOptions::RUN_FROM_GUEST_BINARY) {
            // to remain compatible with a large number of C# tests,
            // we are treating RUN_FROM_GUEST_BINARY as equivalent to
            // RUN_IN_PROCESS | RUN_FROM_GUEST_BINARY
            #[cfg(target_os = "linux")]
            {
                log_then_return!("RUN_FROM_GUEST_BINARY (and RUN_IN_PROCESS by extension) is currently only available on Windows");
            }
            #[cfg(target_os = "windows")]
            {
                if value.contains(SandboxRunOptions::RUN_IN_HYPERVISOR) {
                    log_then_return!(
                        "RUN_FROM_GUEST_BINARY can't be configured with RUN_IN_HYPERVISOR"
                    );
                } else if value.contains(SandboxRunOptions::RECYCLE_AFTER_RUN) {
                    log_then_return!(
                        "RUN_FROM_GUEST_BINARY can't be configured with RECYCLE_AFTER_RUN"
                    );
                }
                Ok(CoreSandboxRunOptions::RunInProcess(true))
            }
        } else {
            log_then_return!("invalid SandboxRunOptions: {:?}", value);
        }
    }
}

impl SandboxRunOptions {
    pub(super) fn should_recycle(&self) -> bool {
        self.contains(SandboxRunOptions::RECYCLE_AFTER_RUN)
    }
}

#[cfg(test)]
mod tests {
    use super::SandboxRunOptions;
    use hyperlight_host::SandboxRunOptions as CoreSandboxRunOptions;

    /// Test the functionality to convert from the FFI-compatible
    /// `SandboxRunOptions` bitfield to the
    /// `hyperlight_host::SandboxRunOptions`
    #[test]
    fn test_convert() {
        let cases = [
            // 0 => RunInHypervisor
            (
                SandboxRunOptions::empty(),
                CoreSandboxRunOptions::RunInHypervisor,
            ),
            // RUN_IN_HYPERVISOR => RunInHypervisor
            (
                SandboxRunOptions::RUN_IN_HYPERVISOR,
                CoreSandboxRunOptions::RunInHypervisor,
            ),
            // RUN_IN_HYPERVISOR | RECYCLE_AFTER_RUN => RunInHypervisor
            (
                SandboxRunOptions::RUN_IN_HYPERVISOR | SandboxRunOptions::RECYCLE_AFTER_RUN,
                CoreSandboxRunOptions::RunInHypervisor,
            ),
            // RECYCLE_AFTER_RUN => RunInHypervisor
            (
                SandboxRunOptions::RECYCLE_AFTER_RUN,
                CoreSandboxRunOptions::RunInHypervisor,
            ),
            #[cfg(target_os = "windows")]
            // RUN_IN_PROCESS => RunInProcess(false)
            // Windows only
            (
                SandboxRunOptions::RUN_IN_PROCESS,
                CoreSandboxRunOptions::RunInProcess(false),
            ),
            #[cfg(target_os = "windows")]
            // RUN_IN_PROCESS | RUN_FROM_GUEST_BINARY => RunInProcess(true)
            // (Windows only)
            (
                SandboxRunOptions::RUN_IN_PROCESS | SandboxRunOptions::RUN_FROM_GUEST_BINARY,
                CoreSandboxRunOptions::RunInProcess(true),
            ),
            #[cfg(target_os = "windows")]
            // RUN_IN_PROCESS | RECYCLE_AFTER_RUN => RunInProcess(false)
            // (Windows only)
            (
                SandboxRunOptions::RUN_IN_PROCESS | SandboxRunOptions::RECYCLE_AFTER_RUN,
                CoreSandboxRunOptions::RunInProcess(false),
            ),
            #[cfg(target_os = "windows")]
            // RUN_IN_PROCESS | RUN_FROM_GUEST_BINARY | RECYCLE_AFTER_RUN => RunInProcess(true)
            // (Windows only)
            (
                SandboxRunOptions::RUN_IN_PROCESS
                    | SandboxRunOptions::RUN_FROM_GUEST_BINARY
                    | SandboxRunOptions::RECYCLE_AFTER_RUN,
                CoreSandboxRunOptions::RunInProcess(true),
            ),
            #[cfg(target_os = "windows")]
            // RUN_FROM_GUEST_BINARY => RunInProcess(true)
            // (for legacy compatibility reasons)
            // (Windows only)
            (
                SandboxRunOptions::RUN_FROM_GUEST_BINARY,
                CoreSandboxRunOptions::RunInProcess(true),
            ),
        ];
        for (orig_cfg, expected_cfg) in cases.iter() {
            let new_cfg: CoreSandboxRunOptions = orig_cfg.clone().try_into().unwrap();
            assert_eq!(new_cfg, expected_cfg.clone());
        }

        // invalid combinations
        {
            let invalid_flags = vec![
                // in-proc and in-hypervisor
                SandboxRunOptions::RUN_IN_HYPERVISOR | SandboxRunOptions::RUN_IN_PROCESS,
                // in-proc, in-hypervisor, from-guest
                SandboxRunOptions::RUN_IN_HYPERVISOR
                    | SandboxRunOptions::RUN_IN_PROCESS
                    | SandboxRunOptions::RUN_FROM_GUEST_BINARY,
            ];
            for invalid_flag in invalid_flags {
                let core_res = CoreSandboxRunOptions::try_from(invalid_flag);
                assert!(core_res.is_err());
            }
        }
    }
}
