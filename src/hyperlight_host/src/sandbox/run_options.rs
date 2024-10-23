use tracing::{instrument, Span};

/// Configuration options for setting up a new `UninitializedSandbox` and
/// subsequent inititialized sandboxes, including `MultiUseSandbox` and
/// `SingleUseSandbox`.
///
/// A `SandboxRunOptions` instance must be created with either in-process
/// or in-hypervisor execution mode, and then can optionally be augmented
/// with run-from-guest-binary mode if created with in-hypervisor mode.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub enum SandboxRunOptions {
    /// Run directly in a platform-appropriate hypervisor
    #[default]
    RunInHypervisor,
    /// Run in-process, without a hypervisor, optionally using the
    /// Windows LoadLibrary API to load the binary if the `bool` field is
    /// set to `true`. This should only be used for testing and debugging
    /// as it does not offer any security guarantees.
    ///
    /// This flag is available on Windows machines only.
    /// There are further plans to make this a debug-only feature.
    /// Please see https://github.com/deislabs/hyperlight/issues/395
    /// for more information
    RunInProcess(bool),
}

impl SandboxRunOptions {
    /// Returns true if the sandbox should be run in-process using the LoadLibrary API.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn use_loadlib(&self) -> bool {
        matches!(self, Self::RunInProcess(true))
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    /// Returns true if the sandbox should be run in-process
    pub(super) fn in_process(&self) -> bool {
        matches!(self, SandboxRunOptions::RunInProcess(_))
    }
}
