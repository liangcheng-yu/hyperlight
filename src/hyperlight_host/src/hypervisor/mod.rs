#[cfg(target_os = "linux")]
///! Functionality to manipulate KVM-based virtual machines.
pub mod kvm;
#[cfg(target_os = "linux")]
///! KVM register definitions
pub mod kvm_regs;
