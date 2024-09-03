use std::fmt::Debug;
use std::sync::OnceLock;

#[cfg(mshv)]
use crate::hypervisor::hyperv_linux;
#[cfg(kvm)]
use crate::hypervisor::kvm;

static AVAILABLE_HYPERVISOR: OnceLock<Option<HypervisorType>> = OnceLock::new();

pub fn get_available_hypervisor() -> &'static Option<HypervisorType> {
    AVAILABLE_HYPERVISOR.get_or_init(|| {
        cfg_if::cfg_if! {
            if #[cfg(all(kvm, mshv))] {
                // If both features are enabled, we need to determine hypervisor at runtime.
                // Currently /dev/kvm and /dev/mshv cannot exist on the same machine, so the first one
                // that works is guaranteed to be correct.
                if hyperv_linux::is_hypervisor_present() {
                    Some(HypervisorType::Mshv)
                } else if kvm::is_hypervisor_present() {
                    Some(HypervisorType::Kvm)
                } else {
                    None
                }
            } else if #[cfg(kvm)] {
                if kvm::is_hypervisor_present() {
                    Some(HypervisorType::Kvm)
                } else {
                    None
                }
            } else if #[cfg(mshv)] {
                if hyperv_linux::is_hypervisor_present() {
                    Some(HypervisorType::Mshv)
                } else {
                    None
                }
            } else if #[cfg(target_os = "windows")] {
                use crate::sandbox::windows_hypervisor_platform;

                if windows_hypervisor_platform::is_hypervisor_present() {
                    Some(HypervisorType::Whp)
                } else {
                    None
                }
            } else {
                None
            }
        }
    })
}

/// The hypervisor types available for the current platform
#[derive(PartialEq, Eq, Debug)]
pub(crate) enum HypervisorType {
    #[cfg(kvm)]
    Kvm,

    #[cfg(mshv)]
    Mshv,

    #[cfg(target_os = "windows")]
    Whp,
}
