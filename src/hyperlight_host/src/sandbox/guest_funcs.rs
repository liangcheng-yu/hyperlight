use std::sync::{Arc, Mutex};

use super::{guest_mgr::GuestMgr, hypervisor::HypervisorWrapperMgr, FunctionsMap};
use crate::{
    func::{
        function_call::{FunctionCall, FunctionCallType},
        guest::GuestFunction,
        param_type::SupportedParameterType,
        types::{ParameterValue, ReturnType},
        HyperlightFunction,
    },
    hypervisor::handlers::{MemAccessHandler, OutBHandler},
    sandbox_state::{reset::RestoreSandbox, sandbox::InitializedSandbox},
};

use anyhow::{bail, Result};
use tracing::instrument;

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
            executing_guest_function.store(0);
        }
    }
}

struct ShouldReset<'a>(bool, &'a mut dyn GuestMgr);

impl<'a> Drop for ShouldReset<'a> {
    fn drop(&mut self) {
        let guest_mgr = &mut self.1;
        guest_mgr.exit_dynamic_method(self.0);
    }
}

/// Enables the host to call functions in the guest and have the sandbox state reset at the start of the call
pub trait CallGuestFunction<'a>:
    GuestMgr + RestoreSandbox + HypervisorWrapperMgr + InitializedSandbox<'a>
{
    fn call_guest_function<T, R>(&mut self, function: T) -> Result<R>
    where
        T: GuestFunction<R>,
    {
        let this = Arc::new(Mutex::new(self));
        // ^^^ needs to be an Arc Mutex because we need multiple owners with mutable
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
            .compare_exchange(0, 1)
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

    #[instrument]
    fn call_guest_function_by_name<P>(
        &mut self,
        name: &str,
        ret: ReturnType,
        args: Option<Vec<P>>,
    ) -> Result<i32>
    where
        P: SupportedParameterType<P> + std::fmt::Debug,
    {
        let this = Arc::new(Mutex::new(self));
        // ^^^ needs to be an Arc Mutex because we need multiple owners with mutable

        let mut guest_mgr = this
            .as_ref()
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?;

        let mut enter_dynamic_method = this
            .as_ref()
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?;

        let should_reset = enter_dynamic_method
            .as_guest_mgr_mut()
            .enter_dynamic_method();

        let mut restore_sandbox = this
            .as_ref()
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?;

        let mut call = this
            .as_ref()
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?;

        // We prefix the variable below w/ an underscore because it is
        // 'technically' unused, as our purpose w/ it is just for it to
        // go out of scope and call its' custom `Drop` `impl`.
        let mut _sr = ShouldReset(should_reset, guest_mgr.as_guest_mgr_mut());

        if should_reset {
            restore_sandbox.reset_state()?;
        }

        let hl_args = args.map(|args| {
            args.into_iter()
                .map(|arg| arg.get_hyperlight_value())
                .collect::<Vec<ParameterValue>>()
        });

        call.dispatch_call_from_host(name, ret, hl_args)
    }

    fn dispatch_call_from_host(
        &mut self,
        function_name: &str,
        return_type: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<i32> {
        let p_dispatch = self.get_mem_mgr().get_pointer_to_dispatch_function()?;

        let fc = FunctionCall::new(
            function_name.to_string(),
            args,
            FunctionCallType::Host,
            return_type,
        );

        let buffer: Vec<u8> = fc.try_into()?;

        self.get_mem_mgr_mut().write_guest_function_call(&buffer)?;

        if self.get_mem_mgr().is_in_process() {
            let dispatch: fn() = unsafe { std::mem::transmute(p_dispatch) };
            // Q: Why does this function not take `args` and doesn't return `return_type`?
            //
            // A: That's because we've already written the function call details to memory
            // with `mem_mgr.write_guest_function_call(&buffer)?;`
            // and the `dispatch` function can directly access that via shared memory.
            dispatch();
        } else {
            // let outb = {
            //     let this_outb = this.clone();

            //     let cb: OutBHandlerFunction<'a> =
            //         Box::new(|port, byte| -> Result<()> {
            //             this_outb
            //                 .lock()
            //                 .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
            //                 .get_initialized_sandbox_mut()
            //                 .handle_outb(port, byte)
            //         });
            //     Arc::new(Mutex::new(OutBHandler::from(cb)))
            // };

            // let mmio_exit = {
            //     let this_mmio_exit = this.clone();

            //     let cb: MemAccessHandlerFunction<'a> = Box::new(|| -> Result<()> {
            //         this_mmio_exit
            //             .lock()
            //             .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?
            //             .get_initialized_sandbox_mut()
            //             .handle_mmio_exit()
            //     });
            //     Arc::new(Mutex::new(MemAccessHandler::from(cb)))
            // };

            let outb_arc = {
                let cb: Box<dyn FnMut(u16, u64) -> Result<()>> = Box::new(|_, _| -> Result<()> {
                    println!("outb callback in test_evolve");
                    Ok(())
                });
                Arc::new(Mutex::new(OutBHandler::from(cb)))
            };
            let mem_access_arc = {
                let cb: Box<dyn FnMut() -> Result<()>> = Box::new(|| -> Result<()> {
                    println!("mem access callback in test_evolve");
                    Ok(())
                });
                Arc::new(Mutex::new(MemAccessHandler::from(cb)))
            };

            self.get_hypervisor_wrapper_mut().dispatch_call_from_host(
                p_dispatch.into(),
                outb_arc.clone(),
                mem_access_arc.clone(),
            )?
        }

        self.check_stack_guard()?; // <- wrapper around mem_mgr `check_for_stack_guard`
        self.get_initialized_sandbox().check_for_guest_error()?;

        self.get_mem_mgr().get_return_value()
    }
}

pub trait GuestFuncs<'a> {
    /// `get_dynamic_methods` is used to get the dynamic guest methods.
    fn get_dynamic_methods(&self) -> &FunctionsMap<'a>;

    /// `get_dynamic_methods_mut` is used to get a mutable reference to the dynamic guest methods.
    fn get_dynamic_methods_mut(&mut self) -> &mut FunctionsMap<'a>;

    /// `add_dynamic_method` is used to register a dynamic guest method onto the Sandbox.
    fn add_dynamic_method(&mut self, name: &str, func: HyperlightFunction<'a>) {
        self.get_dynamic_methods_mut()
            .insert(name.to_string(), func);
    }
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
