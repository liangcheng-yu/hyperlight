use super::guest_err::check_for_guest_error;
use crate::Result;
use crate::{
    func::{
        function_call::{FunctionCall, FunctionCallType},
        types::{ParameterValue, ReturnType, ReturnValue},
    },
    mem::ptr::{GuestPtr, RawPtr},
    HypervisorWrapperMgr, MemMgrWrapperGetter,
};

/// Call a guest function by name, using the given `hv_mem_mgr_getter`.
pub(super) fn dispatch_call_from_host<
    'a,
    HvMemMgrT: HypervisorWrapperMgr<'a> + MemMgrWrapperGetter,
>(
    hv_mem_mgr_getter: &mut HvMemMgrT,
    function_name: &str,
    return_type: ReturnType,
    args: Option<Vec<ParameterValue>>,
) -> Result<ReturnValue> {
    let (is_in_process, p_dispatch) = {
        // only borrow immutably from hv_mem_mgr_getter inside this
        // scope so we can later borrow mutably from it to get the
        // hypervisor
        let mem_mgr = hv_mem_mgr_getter.get_mem_mgr_wrapper();
        (
            mem_mgr.as_ref().is_in_process(),
            mem_mgr.as_ref().get_pointer_to_dispatch_function()?,
        )
    };

    let fc = FunctionCall::new(
        function_name.to_string(),
        args,
        FunctionCallType::Guest,
        return_type,
    );

    let buffer: Vec<u8> = fc.try_into()?;

    {
        // once again, only borrow mutably from hv_mem_mgr_getter
        // from inside this scope so we can borrow mutably later
        let mem_mgr = hv_mem_mgr_getter.get_mem_mgr_wrapper_mut();
        mem_mgr.as_mut().write_guest_function_call(&buffer)?;
    }

    if is_in_process {
        let dispatch: fn() = unsafe { std::mem::transmute(p_dispatch) };
        // Q: Why does this function not take `args` and doesn't return `return_type`?
        //
        // A: That's because we've already written the function call details to memory
        // with `mem_mgr.write_guest_function_call(&buffer)?;`
        // and the `dispatch` function can directly access that via shared memory.
        dispatch();
    } else {
        let p_dispatch_gp = {
            let p_dispatch_rp = RawPtr::from(p_dispatch);
            GuestPtr::try_from(p_dispatch_rp)
        }?;
        // this is the mutable borrow for which we had to do scope gynmastics
        // above
        {
            let hv = hv_mem_mgr_getter.get_hypervisor_wrapper_mut();
            hv.dispatch_call_from_host(p_dispatch_gp)
        }?;
    }

    let mem_mgr = hv_mem_mgr_getter.get_mem_mgr_wrapper();
    mem_mgr.check_stack_guard()?; // <- wrapper around mem_mgr `check_for_stack_guard`
    check_for_guest_error(mem_mgr)?;

    mem_mgr.as_ref().get_function_call_result()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SandboxRunOptions;
    use crate::func::host::HostFunction0;
    use crate::HyperlightError;
    use crate::Result;
    use crate::UninitializedSandbox;
    use crate::{sandbox::is_hypervisor_present, SingleUseSandbox};
    use crate::{sandbox::uninitialized::GuestBinary, sandbox_state::transition::MutatingCallback};
    use crate::{sandbox_state::sandbox::EvolvableSandbox, MultiUseSandbox};
    use hyperlight_testing::callback_guest_path;
    use hyperlight_testing::simple_guest_path;
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    // simple function
    fn test_function0(_: Arc<Mutex<MultiUseSandbox>>) -> Result<i32> {
        Ok(42)
    }

    struct GuestStruct;

    // function that return type unsupported by the host
    fn test_function1(_: SingleUseSandbox) -> Result<GuestStruct> {
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
                None,
            )
            .unwrap()
        };

        // test_function0
        {
            let usbox = uninitialized_sandbox();
            let mut sandbox: MultiUseSandbox<'_> = usbox
                .evolve(MutatingCallback::from(init))
                .expect("Failed to initialize sandbox");
            let func = Arc::new(Mutex::new(test_function0));
            let result = sandbox.execute_in_host(func);
            assert_eq!(result.unwrap(), 42);
        }

        // test_function1
        {
            let usbox = uninitialized_sandbox();
            let sandbox: SingleUseSandbox<'_> = usbox
                .evolve(MutatingCallback::from(init))
                .expect("Failed to initialize sandbox");
            let result = sandbox.execute_in_host(Arc::new(Mutex::new(test_function1)));
            assert!(result.is_ok());
        }

        // test_function2
        {
            let usbox = uninitialized_sandbox();
            let mut sandbox: MultiUseSandbox<'_> = usbox
                .evolve(MutatingCallback::from(init))
                .expect("Failed to initialize sandbox");
            let result = sandbox.execute_in_host(Arc::new(Mutex::new(
                move |_: Arc<Mutex<MultiUseSandbox>>| test_function2(42),
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
                let mut sandbox: MultiUseSandbox<'_> = usbox
                    .evolve(MutatingCallback::from(init))
                    .expect("Failed to initialize sandbox");
                let count = Arc::clone(&count);
                let order = Arc::clone(&order);
                let handle = thread::spawn(move || {
                    let result = sandbox.execute_in_host(Arc::new(Mutex::new(
                        move |_: Arc<Mutex<MultiUseSandbox>>| {
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

    // test_call_guest_function_by_name() but it calls smallVar
    #[test]
    fn test_call_guest_function_by_name_small_var() -> Result<()> {
        // This test relies upon a Hypervisor being present so for now
        // we will skip it if there isnt one.
        if !is_hypervisor_present() {
            println!("Skipping test_call_guest_function_by_name because no hypervisor is present");
            return Ok(());
        }

        let usbox = UninitializedSandbox::new(
            GuestBinary::FilePath(simple_guest_path().expect("Guest Binary Missing")),
            None,
            // ^^^ for now, we're using defaults. In the future, we should get variability here.
            Some(SandboxRunOptions::RunInProcess(true)),
            // ^^^  None == RUN_IN_HYPERVISOR && one-shot Sandbox
            None,
        )?;

        let func = Arc::new(Mutex::new(
            |sbox_arc: Arc<Mutex<MultiUseSandbox>>| -> Result<ReturnValue> {
                let mut sbox = sbox_arc.lock()?;
                sbox.call_guest_function_by_name("small_var", ReturnType::Int, None)
            },
        ));

        let mut sandbox: MultiUseSandbox<'_> = usbox.evolve(MutatingCallback::from(init)).unwrap();

        let result = sandbox.execute_in_host(func).unwrap();

        assert_eq!(result, ReturnValue::Int(2048));
        Ok(())
    }

    #[test]
    fn test_call_guest_function_by_name() -> Result<()> {
        // This test relies upon a Hypervisor being present so for now
        // we will skip it if there isnt one.
        if !is_hypervisor_present() {
            println!("Skipping test_call_guest_function_by_name because no hypervisor is present");
            return Ok(());
        }

        let usbox = UninitializedSandbox::new(
            GuestBinary::FilePath(simple_guest_path().expect("Guest Binary Missing")),
            None,
            // ^^^ for now, we're using defaults. In the future, we should get variability here.
            None,
            // ^^^  None == RUN_IN_HYPERVISOR && one-shot Sandbox
            None,
        )?;

        let msg = "Hello, World!!\n".to_string();
        let len = msg.len() as i32;
        let func = Arc::new(Mutex::new(
            |sbox_arc: Arc<Mutex<MultiUseSandbox>>| -> Result<ReturnValue> {
                let mut sbox = sbox_arc.lock()?;
                sbox.call_guest_function_by_name(
                    "PrintOutput",
                    ReturnType::Int,
                    Some(vec![ParameterValue::String(msg.clone())]),
                )
            },
        ));

        let mut sandbox: MultiUseSandbox<'_> = usbox.evolve(MutatingCallback::from(init)).unwrap();

        let result = sandbox.execute_in_host(func).unwrap();

        assert_eq!(result, ReturnValue::Int(len));
        Ok(())
    }

    // Test that we can terminate a VCPU that has been running the VCPU for too long.
    #[test]
    fn test_terminate_vcpu_spinning_cpu() -> Result<()> {
        // This test relies upon a Hypervisor being present so for now
        // we will skip it if there isnt one.
        if !is_hypervisor_present() {
            println!("Skipping test_call_guest_function_by_name because no hypervisor is present");
            return Ok(());
        }
        let usbox = UninitializedSandbox::new(
            GuestBinary::FilePath(simple_guest_path().expect("Guest Binary Missing")),
            None,
            None,
            None,
        )?;

        let sandbox = {
            let new_sbox: MultiUseSandbox = usbox.evolve(MutatingCallback::from(init))?;
            Arc::new(Mutex::new(new_sbox))
        };
        let func = {
            let f = move |s: Arc<Mutex<MultiUseSandbox>>| -> Result<ReturnValue> {
                println!(
                    "Calling Guest Function Spin - this should be cancelled by the host after 1000ms"
                );
                s.lock()?
                    .call_guest_function_by_name("Spin", ReturnType::Void, None)
            };
            Arc::new(Mutex::new(f))
        };

        let result = sandbox.lock()?.execute_in_host(func);

        assert!(result.is_err());
        match result.unwrap_err() {
            HyperlightError::ExecutionCanceledByHost() => {}
            e => panic!(
                "Expected HyperlightError::ExecutionCanceledByHost() but got {:?}",
                e
            ),
        }
        Ok(())
    }
    // This test is to capture the case where the guest execution is running a hsot function when cancelled and that host function
    // is never going to return.
    // The host function that is called will end after 5 seconds, but by this time the cancellation will have given up
    // (using default timeout settings)  , so this tests looks for the error "Failed to cancel guest execution".
    // Eventually once we fix https://github.com/deislabs/hyperlight/issues/951 this test should be updated.

    #[test]
    fn test_terminate_vcpu_calling_host_spinning_cpu() -> Result<()> {
        // This test relies upon a Hypervisor being present so for now
        // we will skip it if there isnt one.
        if !is_hypervisor_present() {
            println!("Skipping test_call_guest_function_by_name because no hypervisor is present");
            return Ok(());
        }
        let mut usbox = UninitializedSandbox::new(
            GuestBinary::FilePath(callback_guest_path().expect("Guest Binary Missing")),
            None,
            None,
            None,
        )?;

        // Make this host call run for 5 seconds

        fn spin() -> Result<()> {
            thread::sleep(std::time::Duration::from_secs(5));
            Ok(())
        }

        let host_spin_func = Arc::new(Mutex::new(spin));

        host_spin_func.register(&mut usbox, "Spin")?;

        let sandbox = {
            let new_sbox: MultiUseSandbox = usbox.evolve(MutatingCallback::from(init))?;
            Arc::new(Mutex::new(new_sbox))
        };
        let func = Arc::new(Mutex::new(
            move |s: Arc<Mutex<MultiUseSandbox>>| -> Result<ReturnValue> {
                println!(
                    "Calling Guest Function CallHostSpin - this should fail to cancel the guest execution after 5 seconds"
                );
                s.lock()?
                    .call_guest_function_by_name("CallHostSpin", ReturnType::Void, None)
            },
        ));

        let result = sandbox.lock()?.execute_in_host(func);
        assert!(result.is_err());
        match result.unwrap_err() {
            HyperlightError::HostFailedToCancelGuestExecution() => {}
            #[cfg(target_os = "linux")]
            HyperlightError::HostFailedToCancelGuestExecutionSendingSignals(_) => {}
            e => panic!("Unexpected Error got {:?}", e),
        }
        Ok(())
    }
}
