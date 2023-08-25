/// Functionality for interacting with guest calls
pub(crate) mod guest_funcs;
/// Functionality for managing the guest
pub(crate) mod guest_mgr;
/// Functionality for reading, but not modifying host functions
pub mod host_funcs;
/// Functionality for dealing with `Sandbox`es that contain Hypervisors
pub(crate) mod hypervisor;
/// Functionality for dealing with completely initialized sandboxes
pub mod initialized;
/// Functionality for dealing with memory access from the VM guest
/// executable
mod mem_access;
/// Functionality for interacting with a sandbox's internally-stored
/// `SandboxMemoryManager`
pub mod mem_mgr;
mod outb;
/// Options for configuring a sandbox
mod run_options;
/// Functionality for creating uninitialized sandboxes, manipulating them,
/// and converting them to initialized sandboxes.
pub mod uninitialized;
/// Functionality for properly converting `UninitailizedSandbox`es to
/// initialized `Sandbox`es.
mod uninitialized_evolve;

/// Re-export for `CallGuestFunction` trait
pub use guest_funcs::CallGuestFunction;
/// Re-export for `GuestMgr` trait
pub use guest_mgr::GuestMgr;
/// Re-export for `HypervisorWrapper` trait
pub use hypervisor::HypervisorWrapper;
/// Re-export for `HypervisorWrapperMgr` type
pub use hypervisor::HypervisorWrapperMgr;
/// Re-export for `ExecutingGuestCall` type
pub use initialized::ExecutingGuestCall;
/// Re-export for `Sandbox` type
pub use initialized::Sandbox;
/// Re-export for `MemMgrWrapper` type
pub use mem_mgr::MemMgrWrapper;
/// Re-export for `MemMgrWrapperGetter` trait
pub use mem_mgr::MemMgrWrapperGetter;
/// Re-export for `SandboxRunOptions` type
pub use run_options::SandboxRunOptions;
/// Re-export for `GuestBinary` type
pub use uninitialized::GuestBinary;
/// Re-export for `UninitializedSandbox` type
pub use uninitialized::UninitializedSandbox;

use crate::func::HyperlightFunction;
#[cfg(target_os = "windows")]
use crate::hypervisor::windows_hypervisor_platform;
#[cfg(target_os = "linux")]
use crate::{hypervisor::hyperv_linux, hypervisor::kvm};
use std::collections::HashMap;

// In case its not obvious why there are separate is_supported_platform and is_hypervisor_present functions its because
// Hyperlight is designed to be able to run on a host that doesn't have a hypervisor.
// In that case, the sandbox will be in process, we plan on making this a dev only feature and fixing up Linux support
// so we should review the need for this function at that time.

/// Determine if this is a supported platform for Hyperlight
///
/// Returns a boolean indicating whether this is a supported platform.
pub fn is_supported_platform() -> bool {
    #[cfg(not(target_os = "linux"))]
    #[cfg(not(target_os = "windows"))]
    return false;

    true
}

/// A `HashMap` to map function names to `HyperlightFunction`s.
#[derive(Clone, Default)]
pub struct FunctionsMap<'a>(HashMap<String, HyperlightFunction<'a>>);

impl<'a> FunctionsMap<'a> {
    /// Insert a new entry into the map.
    pub fn insert(&mut self, key: String, value: HyperlightFunction<'a>) {
        self.0.insert(key, value);
    }

    /// Get the value associated with the given key, if it exists.
    pub fn get(&self, key: &str) -> Option<&HyperlightFunction<'a>> {
        self.0.get(key)
    }

    /// Get the length of the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the map is empty, otherwise false
    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }
}

impl<'a> PartialEq for FunctionsMap<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.0.keys().all(|k| other.0.contains_key(k))
    }
}

impl<'a> Eq for FunctionsMap<'a> {}

/// Determine whether a suitable hypervisor is available to run
/// this sandbox.
///
//  Returns a boolean indicating whether a suitable hypervisor is present.
pub fn is_hypervisor_present() -> bool {
    #[cfg(target_os = "linux")]
    {
        hyperv_linux::is_hypervisor_present().unwrap_or(false)
            || kvm::is_hypervisor_present().is_ok()
    }
    #[cfg(target_os = "windows")]
    {
        windows_hypervisor_platform::is_hypervisor_present().unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    #[cfg(not(target_os = "windows"))]
    false
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::is_hypervisor_present;
    #[cfg(target_os = "linux")]
    use crate::hypervisor::hyperv_linux::test_cfg::TEST_CONFIG as HYPERV_TEST_CONFIG;
    #[cfg(target_os = "linux")]
    use crate::hypervisor::kvm::test_cfg::TEST_CONFIG as KVM_TEST_CONFIG;
    use crate::Sandbox;
    use crate::{sandbox::uninitialized::GuestBinary, sandbox_state::transition::Noop};
    use crate::{sandbox_state::sandbox::EvolvableSandbox, UninitializedSandbox};
    use crossbeam_queue::ArrayQueue;
    use hyperlight_testing::simple_guest_path;
    use std::{sync::Arc, thread};

    #[test]
    // TODO: add support for testing on WHP
    #[cfg(target_os = "linux")]
    fn test_is_hypervisor_present() {
        // TODO: Handle requiring a stable API
        if HYPERV_TEST_CONFIG.hyperv_should_be_present || KVM_TEST_CONFIG.kvm_should_be_present {
            assert!(is_hypervisor_present());
        } else {
            assert!(!is_hypervisor_present());
        }
    }

    #[test]
    fn check_create_and_use_sandbox_on_different_threads() {
        let unintializedsandbox_queue = Arc::new(ArrayQueue::<UninitializedSandbox>::new(10));
        let sandbox_queue = Arc::new(ArrayQueue::<Sandbox>::new(10));

        for i in 0..10 {
            let simple_guest_path = simple_guest_path().expect("Guest Binary Missing");
            let unintializedsandbox =
                UninitializedSandbox::new(GuestBinary::FilePath(simple_guest_path), None, None)
                    .unwrap_or_else(|_| panic!("Failed to create UninitializedSandbox {}", i));

            unintializedsandbox_queue
                .push(unintializedsandbox)
                .unwrap_or_else(|_| panic!("Failed to push UninitializedSandbox {}", i));
        }

        let thread_handles = (0..10)
            .map(|i| {
                let uq = unintializedsandbox_queue.clone();
                let sq = sandbox_queue.clone();
                thread::spawn(move || {
                    let uninitialized_sandbox = uq.pop().unwrap_or_else(|| {
                        panic!("Failed to pop UninitializedSandbox thread {}", i)
                    });
                    let host_funcs = uninitialized_sandbox.host_funcs.lock();

                    assert!(host_funcs.is_ok());

                    host_funcs
                        .unwrap()
                        .host_print(format!("Print from UninitializedSandbox on Thread {}\n", i))
                        .unwrap();

                    let sandbox = uninitialized_sandbox
                        .evolve(Noop::default())
                        .unwrap_or_else(|_| {
                            panic!("Failed to initialize UninitializedSandbox thread {}", i)
                        });

                    sq.push(sandbox).unwrap_or_else(|_| {
                        panic!("Failed to push UninitializedSandbox thread {}", i)
                    })
                })
            })
            .collect::<Vec<_>>();

        for handle in thread_handles {
            handle.join().unwrap();
        }

        let thread_handles = (0..10)
            .map(|i| {
                let sq = sandbox_queue.clone();
                thread::spawn(move || {
                    let sandbox = sq
                        .pop()
                        .unwrap_or_else(|| panic!("Failed to pop Sandbox thread {}", i));
                    let host_funcs = sandbox.host_functions.lock();

                    assert!(host_funcs.is_ok());

                    host_funcs
                        .unwrap()
                        .host_print(format!("Print from Sandbox on Thread {}\n", i))
                        .unwrap();
                })
            })
            .collect::<Vec<_>>();

        for handle in thread_handles {
            handle.join().unwrap();
        }
    }
}
