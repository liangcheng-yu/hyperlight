use super::{guest_mgr::GuestMgr, hypervisor::HypervisorWrapperMgr, mem_mgr::MemMgrWrapperGetter};
use std::sync::{Arc, Mutex};

use crate::{
    func::{
        function_call::{FunctionCall, FunctionCallType},
        guest::GuestFunction,
        types::{ParameterValue, ReturnType},
    },
    mem::ptr::{GuestPtr, RawPtr},
    sandbox_state::{reset::RestoreSandbox, sandbox::InitializedSandbox},
    Sandbox,
};
use anyhow::{bail, Result};
use tracing::instrument;

// `ShouldRelease` is an internal construct that represents a
// port of try-finally logic in C#.
//
// It implements `drop` and captures part of our state in
// `call_guest_function`, to allow it to properly act
// on it and do cleanup.
struct ShouldRelease<'a, 'b>(bool, Arc<Mutex<&'b mut Sandbox<'a>>>);

impl<'a, 'b> ShouldRelease<'a, 'b> {
    #[allow(unused)]
    fn toggle(&mut self) {
        self.0 = !self.0;
    }
}

impl<'a, 'b> Drop for ShouldRelease<'a, 'b> {
    fn drop(&mut self) {
        if self.0 {
            let sbox = &mut self.1.lock().unwrap();
            sbox.set_needs_state_reset(true);
            let executing_guest_function = sbox.get_executing_guest_call_mut();
            executing_guest_function.store(0);
        }
    }
}

struct ShouldReset<'a, 'b>(bool, Arc<Mutex<&'b mut Sandbox<'a>>>);

impl<'a, 'b> Drop for ShouldReset<'a, 'b> {
    fn drop(&mut self) {
        let mut sbox = self.1.lock().unwrap();
        sbox.exit_method(self.0);
    }
}

/// Enables the host to call functions in the guest and have the sandbox state reset at the start of the call
pub trait CallGuestFunction<'a>:
    GuestMgr
    + RestoreSandbox<'a>
    + HypervisorWrapperMgr<'a>
    + MemMgrWrapperGetter
    + InitializedSandbox<'a>
{
    fn execute_in_host<T, R>(&mut self, function: T) -> Result<R>
    where
        T: GuestFunction<R>,
    {
        let sbox = Arc::new(Mutex::new(self.get_initialized_sandbox_mut()));

        // We prefix the variable below w/ an underscore because it is
        // 'technically' unused, as our purpose w/ it is just for it to
        // go out of scope and call its' custom `Drop` `impl`.
        let mut _sd = ShouldRelease(false, sbox.clone());
        if sbox
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
            .get_executing_guest_call_mut()
            .compare_exchange(0, 1)
            .map_err(|_| anyhow::anyhow!("Failed to verify status of guest function execution"))?
            != 0
        {
            bail!("Guest call already in progress");
        }

        _sd.toggle();
        sbox.lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
            .reset_state()?;

        function.call(sbox.clone())
        // ^^^ ensures that only one call can be made concurrently
        // because `GuestFunction` is implemented for `Arc<Mutex<T>>`
        // so we'll be locking on the function call. There are tests
        // below that demonstrate this.
    }

    #[instrument]
    fn call_guest_function_by_name(
        &mut self,
        name: &str,
        ret: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<i32> {
        let sbox = Arc::new(Mutex::new(self.get_initialized_sandbox_mut()));

        let should_reset = sbox
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
            .as_guest_mgr_mut()
            .enter_method();

        // We prefix the variable below w/ an underscore because it is
        // 'technically' unused, as our purpose w/ it is just for it to
        // go out of scope and call its' custom `Drop` `impl`.
        let mut _sr = ShouldReset(should_reset, sbox.clone());

        if should_reset {
            sbox.lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
                .reset_state()?;
        }

        let mut dispatcher = sbox
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?;

        dispatcher.dispatch_call_from_host(name, ret, args)
    }

    fn dispatch_call_from_host(
        &mut self,
        function_name: &str,
        return_type: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<i32> {
        let mem_mgr = self.get_mem_mgr_wrapper().as_ref();
        let p_dispatch = mem_mgr.get_pointer_to_dispatch_function()?;

        let fc = FunctionCall::new(
            function_name.to_string(),
            args,
            FunctionCallType::Host,
            return_type,
        );

        let buffer: Vec<u8> = fc.try_into()?;

        self.get_mem_mgr_wrapper_mut()
            .as_mut()
            .write_guest_function_call(&buffer)?;

        if self.get_mem_mgr_wrapper().as_ref().is_in_process() {
            let dispatch: fn() = unsafe { std::mem::transmute(p_dispatch) };
            // Q: Why does this function not take `args` and doesn't return `return_type`?
            //
            // A: That's because we've already written the function call details to memory
            // with `mem_mgr.write_guest_function_call(&buffer)?;`
            // and the `dispatch` function can directly access that via shared memory.
            dispatch();
        } else {
            let sbox = Arc::new(Mutex::new(self.get_initialized_sandbox_mut()));

            let p_dispatch_gp = {
                let p_dispatch_rp = RawPtr::from(p_dispatch);
                GuestPtr::try_from(p_dispatch_rp)
            }?;
            sbox.lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
                .get_hypervisor_wrapper_mut()
                .dispatch_call_from_host(p_dispatch_gp)?;
        }

        self.get_mem_mgr_wrapper().check_stack_guard()?; // <- wrapper around mem_mgr `check_for_stack_guard`
        self.get_initialized_sandbox().check_for_guest_error()?;

        self.get_mem_mgr_wrapper().as_ref().get_return_value()
    }
}

#[cfg(test)]
mod tests {
    use crate::UninitializedSandbox;
    use crate::{sandbox::uninitialized::GuestBinary, sandbox_state::transition::MutatingCallback};
    use crate::{sandbox_state::sandbox::EvolvableSandbox, testing::simple_guest_path};

    use super::*;
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    // simple function
    fn test_function0(_: Arc<Mutex<&mut Sandbox>>) -> Result<i32> {
        Ok(42)
    }

    struct GuestStruct;

    // function that return type unsupported by the host
    fn test_function1(_: Arc<Mutex<&mut Sandbox>>) -> Result<GuestStruct> {
        Ok(GuestStruct)
    }

    // function that takes a parameter
    fn test_function2(param: i32) -> Result<i32> {
        Ok(param)
    }

    // blank convenience init function for transitioning between a usbox and a isbox
    fn init(_: &mut UninitializedSandbox) -> Result<()> {
        Ok(())
    }

    #[test]
    fn test_execute_in_host() {
        let uninitialized_sandbox = || {
            UninitializedSandbox::new(
                GuestBinary::FilePath(simple_guest_path().expect("Guest Binary Missing")),
                None,
                None,
            )
            .unwrap()
        };

        // test_function0
        {
            let usbox = uninitialized_sandbox();
            let mut sandbox = usbox
                .evolve(MutatingCallback::from(init))
                .expect("Failed to initialize sandbox");
            let result = sandbox.execute_in_host(Arc::new(Mutex::new(test_function0)));
            assert_eq!(result.unwrap(), 42);
        }

        // test_function1
        {
            let usbox = uninitialized_sandbox();
            let mut sandbox = usbox
                .evolve(MutatingCallback::from(init))
                .expect("Failed to initialize sandbox");
            let result = sandbox.execute_in_host(Arc::new(Mutex::new(test_function1)));
            assert!(result.is_ok());
        }

        // test_function2
        {
            let usbox = uninitialized_sandbox();
            let mut sandbox = usbox
                .evolve(MutatingCallback::from(init))
                .expect("Failed to initialize sandbox");
            let result = sandbox.execute_in_host(Arc::new(Mutex::new(
                move |_: Arc<Mutex<&mut Sandbox>>| test_function2(42),
            )));
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
                    .evolve(MutatingCallback::from(init))
                    .expect("Failed to initialize sandbox");
                let count = Arc::clone(&count);
                let order = Arc::clone(&order);
                let handle = thread::spawn(move || {
                    let result = sandbox.execute_in_host(Arc::new(Mutex::new(
                        move |_: Arc<Mutex<&mut Sandbox>>| {
                            let mut num = count.lock().unwrap();
                            *num += 1;
                            Ok(*num)
                        },
                    )));
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
