#[cfg(target_os = "linux")]
///! HyperV-on-linux functionality
pub mod hyperv_linux;
#[cfg(target_os = "linux")]
///! HyperV-on-linux memory utilities
pub mod hyperv_linux_mem;
#[cfg(target_os = "linux")]
///! Functionality to manipulate KVM-based virtual machines.
pub mod kvm;
#[cfg(target_os = "linux")]
///! Memory management functions for KVM
pub mod kvm_mem;
#[cfg(target_os = "linux")]
///! KVM register definitions
pub mod kvm_regs;
#[cfg(target_os = "windows")]
///! Hyperlight Surrogate Process
pub(crate) mod surrogate_process;
#[cfg(target_os = "windows")]
///! Hyperlight Surrogate Process
pub(crate) mod surrogate_process_manager;
