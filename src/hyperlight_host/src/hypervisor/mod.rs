#[cfg(target_os = "linux")]
use mshv_bindings::{hv_register_value, hv_u128};

#[cfg(target_os = "linux")]
///! HyperV-on-linux functionality
pub mod hyperv_linux;
#[cfg(target_os = "linux")]
///! Functionality to manipulate KVM-based virtual machines.
pub mod kvm;
#[cfg(target_os = "linux")]
///! Memory management functions for KVM
pub mod kvm_mem;
#[cfg(target_os = "linux")]
///! KVM register definitions
pub mod kvm_regs;

/// A representation of an unsigned 128-bit integer,
/// used in various register APIs herein
pub struct U128 {
    /// The less significant 64 bits of this 128-bit integer
    pub low: u64,
    /// The more significant 64 bits of this 128-bit integer
    pub high: u64,
}

#[cfg(target_os = "linux")]
impl From<U128> for hv_u128 {
    fn from(val: U128) -> Self {
        hv_u128 {
            low_part: val.low,
            high_part: val.high,
        }
    }
}

#[cfg(target_os = "linux")]
impl From<U128> for hv_register_value {
    fn from(val: U128) -> Self {
        hv_register_value {
            reg128: hv_u128::from(val),
        }
    }
}

impl From<u64> for U128 {
    fn from(val: u64) -> Self {
        U128 { low: val, high: 0 }
    }
}
