use anyhow::Result;
use std::sync::{atomic::AtomicI32, Arc};

/// A container to atomically keep track of whether a sandbox is currently executing a guest call. Primarily
/// used to prevent concurrent execution of guest calls.
///
/// 0 = not executing a guest call
/// 1 = executing `execute_in_host`
/// 2 = executing a `call_guest_function_by_name`
#[derive(Clone, Debug)]
pub struct ExecutingGuestCall(Arc<AtomicI32>);

impl ExecutingGuestCall {
    /// Create a new `ExecutingGuestCall` with the provided value.
    pub fn new(val: i32) -> Self {
        Self(Arc::new(AtomicI32::new(val)))
    }

    /// Load the value of the `ExecutingGuestCall`.
    pub fn load(&self) -> i32 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Store a value in the `ExecutingGuestCall`.
    pub fn store(&self, val: i32) {
        self.0.store(val, std::sync::atomic::Ordering::SeqCst);
    }

    /// Compare and exchange the value of the `ExecutingGuestCall`.
    pub fn compare_exchange(&self, current: i32, new: i32) -> Result<i32> {
        self.0
            .compare_exchange(
                current,
                new,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .map_err(|_| anyhow::anyhow!("compare_exchange failed"))
    }
}

impl PartialEq for ExecutingGuestCall {
    fn eq(&self, other: &Self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
            == other.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Eq for ExecutingGuestCall {}
