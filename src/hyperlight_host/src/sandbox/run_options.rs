/// Configuration options for setting up a new `UninitializedSandbox` and
/// subsequent inititialized sandboxes, including `MultiUseSandbox` and
/// `SingleUseSandbox`.
///
/// A `SandboxRunOptions` instance must be created with either in-process
/// or in-hypervisor execution mode, and then can optionally be augmented
/// with run-from-guest-binary mode if created with in-hypervisor mode.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub enum SandboxRunOptions {
    /// Run directly in a platform-appropriate hypervisor, with the option
    /// to re-use sandboxes after each execution if the `bool` field is
    /// set to `true`.
    #[default]
    RunInHypervisor,
    #[cfg(target_os = "windows")]
    /// Run in-process, without a hypervisor, optionally using the
    /// Windows LoadLibrary API to load the binary if the `bool` field is
    /// set to `true`.
    ///
    /// This flag is available on Windows machines only.
    /// There are further plans to make this a debug-only feature.
    /// Please see https://github.com/deislabs/hyperlight/issues/395
    /// for more information
    RunInProcess(bool),
}

impl SandboxRunOptions {
    pub(super) fn is_run_from_guest_binary(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            false
        }
        #[cfg(target_os = "windows")]
        {
            matches!(self, Self::RunInProcess(true))
        }
    }
    pub(super) fn is_in_memory(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            false
        }
        #[cfg(target_os = "windows")]
        {
            matches!(self, SandboxRunOptions::RunInProcess(_))
        }
    }
}
