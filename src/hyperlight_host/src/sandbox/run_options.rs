use bitflags::bitflags;

// TODO: Prevent inprocess and run from guest binary in release builds

bitflags! {
    #[repr(C)]
    #[derive(Debug)]
    /// Options for running a sandbox
    pub struct SandboxRunOptions: u32 {
        /// Run in a Hypervisor
        const RUN_IN_HYPERVISOR = 0b00000000;
        /// Run in process (windows only)
        const RUN_IN_PROCESS = 0b00000001;
        /// Recycle the sandbox after running
        const RECYCLE_AFTER_RUN = 0b00000010;
        /// Run from guest binary (windows only)
        #[cfg(os = "windows")]
        const RUN_FROM_GUEST_BINARY =0b00000100;
    }
}
