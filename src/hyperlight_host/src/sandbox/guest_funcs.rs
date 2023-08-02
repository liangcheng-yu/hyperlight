use std::sync::{atomic::Ordering, Arc, Mutex};

use super::guest_mgr::GuestMgr;
use crate::{func::guest::GuestFunction, sandbox_state::reset::RestoreSandbox};

use anyhow::{bail, Result};

// `ShouldRelease` is an internal construct that represents a
// port of try-finally logic in C#.
//
// It implements `drop` and captures part of our state in
// `call_guest_function`, to allow it to properly act
// on it and do cleanup.
struct ShouldRelease<'a>(bool, &'a mut dyn GuestMgr);

impl<'a> ShouldRelease<'a> {
    #[allow(unused)]
    fn toggle(&mut self) {
        self.0 = !self.0;
    }
}

impl<'a> Drop for ShouldRelease<'a> {
    fn drop(&mut self) {
        if self.0 {
            let guest_mgr = &mut self.1;
            guest_mgr.set_needs_state_reset(true);
            let executing_guest_function = guest_mgr.get_executing_guest_call_mut();
            executing_guest_function.store(0, Ordering::SeqCst);
        }
    }
}

/// Enables the host to call functions in the guest and have the sandbox state reset at the start of the call
pub(crate) trait CallGuestFunction<'a>: GuestMgr + RestoreSandbox {
    fn call_guest_function<T, R>(&mut self, function: T) -> Result<R>
    where
        T: GuestFunction<R>,
    {
        let this = Arc::new(Mutex::new(self));
        // ^^^ needs to be an Arc Mutex because we need three owners with mutable
        // access in a thread-safe way, as highlighted below:

        let mut guest_mgr = this
            .as_ref()
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?;

        let mut executing_guest_function = this
            .as_ref()
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?;

        let mut restore_sandbox = this
            .as_ref()
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?;

        // We prefix the variable below w/ an underscore because it is
        // 'technically' unused, as our purpose w/ it is just for it to
        // go out of scope and call its' custom `Drop` `impl`.
        let mut _sd = ShouldRelease(false, guest_mgr.as_guest_mgr_mut());
        if executing_guest_function
            .get_executing_guest_call_mut()
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| anyhow::anyhow!("Failed to verify status of guest function execution"))?
            != 0
        {
            bail!("Guest call already in progress");
        }

        _sd.toggle();
        restore_sandbox.reset_state()?;
        function.call()
        // ^^^ ensures that only one call can be made concurrently
        // because `GuestFunction` is implemented for `Arc<Mutex<T>>`
        // so we'll be locking on the function call. There are tests
        // below that demonstrate this.
    }

    /// `enter_dynamic_method` is used to indicate if a `Sandbox`'s state should be reset.
    /// - When we enter call a guest function, the `executing_guest_call` value is set to 1.
    /// - When we exit a guest function, the `executing_guest_call` value is set to 0.
    ///
    /// `enter_dynamic_method` will check if the value of `executing_guest_call` is 1.
    /// If yes, it means the guest function is still running and state should not be reset.
    /// If the value of `executing_guest_call` is 0, we should reset the state.
    fn enter_dynamic_method(&mut self) -> Result<bool> {
        let executing_guest_function = self.get_executing_guest_call_mut();
        if executing_guest_function.load(Ordering::SeqCst) == 1 {
            return Ok(false);
        }

        if executing_guest_function
            .compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| anyhow::anyhow!("Failed to verify status of guest function execution"))?
            != 0
        {
            bail!("Guest call already in progress");
        }

        Ok(true)
    }

    /// `exit_dynamic_method` is used to indicate that a guest function has finished executing.
    fn exit_dynamic_method(&mut self, should_release: bool) -> Result<()> {
        if should_release {
            self.get_executing_guest_call_mut()
                .store(0, Ordering::SeqCst);
            self.set_needs_state_reset(true);
        }

        Ok(())
    }

    // TODO: add `call_dynamic_guest_func`
}

#[cfg(test)]
mod tests {
    use crate::testing::simple_guest_path;
    use crate::UninitializedSandbox;

    use super::*;
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    // simple function
    fn test_function0() -> Result<i32> {
        Ok(42)
    }

    struct GuestStruct;

    // function that return type unsupported by the host
    fn test_function1() -> Result<GuestStruct> {
        Ok(GuestStruct)
    }

    // function that takes a parameter
    fn test_function2(param: i32) -> Result<i32> {
        Ok(param)
    }

    #[test]
    #[ignore]
    fn test_call_guest_function() {
        let uninitialized_sandbox = || {
            UninitializedSandbox::new(
                simple_guest_path().expect("Guest Binary Missing"),
                None,
                None,
            )
            .unwrap()
        };
        fn init(_: &mut UninitializedSandbox) -> Result<()> {
            Ok(())
        }

        // test_function0
        {
            let usbox = uninitialized_sandbox();
            let mut sandbox = usbox
                .initialize(Some(init))
                .expect("Failed to initialize sandbox");
            let result = sandbox.call_guest_function(Arc::new(Mutex::new(test_function0)));
            assert_eq!(result.unwrap(), 42);
        }

        // test_function1
        {
            let usbox = uninitialized_sandbox();
            let mut sandbox = usbox
                .initialize(Some(init))
                .expect("Failed to initialize sandbox");
            let result = sandbox.call_guest_function(Arc::new(Mutex::new(test_function1)));
            assert!(result.is_ok());
        }

        // test_function2
        {
            let usbox = uninitialized_sandbox();
            let mut sandbox = usbox
                .initialize(Some(init))
                .expect("Failed to initialize sandbox");
            let result =
                sandbox.call_guest_function(Arc::new(Mutex::new(move || test_function2(42))));
            assert_eq!(result.unwrap(), 42);
        }

        // test concurrent calls with a local closure that returns current count
        {
            let count = Arc::new(Mutex::new(0));
            let order = Arc::new(Mutex::new(vec![]));

            let mut handles = vec![];

            for _ in 0..10 {
                let usbox = uninitialized_sandbox();
                let mut sandbox = usbox
                    .initialize(Some(init))
                    .expect("Failed to initialize sandbox");
                let count = Arc::clone(&count);
                let order = Arc::clone(&order);
                let handle = thread::spawn(move || {
                    let result = sandbox.call_guest_function(Arc::new(Mutex::new(move || {
                        let mut num = count.lock().unwrap();
                        *num += 1;
                        Ok(*num)
                    })));
                    order.lock().unwrap().push(result.unwrap());
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }

            // Check if the order of operations is sequential
            let order = order.lock().unwrap();
            for i in 0..10 {
                assert_eq!(order[i], i + 1);
            }
        }

        // TODO: Add tests to ensure State has been reset.
    }
}
