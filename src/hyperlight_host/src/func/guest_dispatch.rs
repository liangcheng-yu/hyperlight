use super::guest_err::check_for_guest_error;
#[cfg(feature = "function_call_metrics")]
use crate::histogram_vec_time_micros;
use crate::mem::ptr::RawPtr;
#[cfg(feature = "function_call_metrics")]
use crate::sandbox::metrics::SandboxMetric::GuestFunctionCallDurationMicroseconds;
use crate::sandbox::WrapperGetter;
use crate::{HyperlightError, Result};
use cfg_if::cfg_if;
use hyperlight_common::flatbuffer_wrappers::{
    function_call::{FunctionCall, FunctionCallType},
    function_types::{ParameterValue, ReturnType, ReturnValue},
};
use tracing::{instrument, Span};

/// Call a guest function by name, using the given `wrapper_getter`.
#[instrument(
    err(Debug),
    skip(wrapper_getter, args),
    parent = Span::current(),
    level = "Trace"
)]
pub(crate) fn call_function_on_guest<'a, HvMemMgrT: WrapperGetter<'a>>(
    wrapper_getter: &mut HvMemMgrT,
    function_name: &str,
    return_type: ReturnType,
    args: Option<Vec<ParameterValue>>,
) -> Result<ReturnValue> {
    let (is_in_process, p_dispatch) = {
        // only borrow immutably from hv_mem_mgr_getter inside this
        // scope so we can later borrow mutably from it to get the
        // hypervisor
        let mem_mgr = wrapper_getter.get_mgr().as_ref();
        (
            mem_mgr.is_in_process(),
            mem_mgr.get_pointer_to_dispatch_function()?,
        )
    };

    let fc = FunctionCall::new(
        function_name.to_string(),
        args,
        FunctionCallType::Guest,
        return_type,
    );

    let buffer: Vec<u8> = fc
        .try_into()
        .map_err(|_| HyperlightError::Error("Failed to serialize FunctionCall".to_string()))?;

    {
        // once again, only borrow mutably from hv_mem_mgr_getter
        // from inside this scope so we can borrow mutably later
        let mem_mgr = wrapper_getter.get_mgr_mut();
        mem_mgr.as_mut().write_guest_function_call(&buffer)?;
    }

    if is_in_process {
        let dispatch: fn() = unsafe { std::mem::transmute(p_dispatch) };
        // Q: Why does this function not take `args` and doesn't return `return_type`?
        //
        // A: That's because we've already written the function call details to memory
        // with `mem_mgr.write_guest_function_call(&buffer)?;`
        // and the `dispatch` function can directly access that via shared memory.
        cfg_if! {
            if #[cfg(feature = "function_call_metrics")] {
                histogram_vec_time_micros!(
                    &GuestFunctionCallDurationMicroseconds,
                    &[function_name],
                    dispatch()
                );
            }
            else {
                dispatch();
            }
        }
    } else {
        // this is the mutable borrow for which we had to do scope gynmastics
        // above
        {
            let hv_wrapper = wrapper_getter.get_hv_mut();
            let mut hv = hv_wrapper.get_hypervisor()?;
            cfg_if! {
                if #[cfg(feature = "function_call_metrics")] {
                    histogram_vec_time_micros!(
                        &GuestFunctionCallDurationMicroseconds,
                        &[function_name],
                        hv.dispatch_call_from_host(
                            RawPtr::from(p_dispatch),
                            hv_wrapper.outb_hdl.clone(),
                            hv_wrapper.mem_access_hdl.clone(),
                            hv_wrapper.max_execution_time,
                            hv_wrapper.max_wait_for_cancellation,
                        )
                    )
                }
                else {
                    hv.dispatch_call_from_host(
                        RawPtr::from(p_dispatch),
                        hv_wrapper.outb_hdl.clone(),
                        hv_wrapper.mem_access_hdl.clone(),
                        hv_wrapper.max_execution_time,
                        hv_wrapper.max_wait_for_cancellation,
                    )
                }
            }
        }?;
    }

    let mem_mgr = wrapper_getter.get_mgr_mut();
    mem_mgr.check_stack_guard()?; // <- wrapper around mem_mgr `check_for_stack_guard`
    check_for_guest_error(mem_mgr)?;

    mem_mgr.as_mut().get_guest_function_call_result()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::{
        call_ctx::{MultiUseGuestCallContext, SingleUseGuestCallContext},
        host_functions::HostFunction0,
    };
    use crate::HyperlightError;
    use crate::Result;
    use crate::UninitializedSandbox;
    use crate::{sandbox::is_hypervisor_present, SingleUseSandbox};
    use crate::{sandbox::uninitialized::GuestBinary, sandbox_state::transition::MutatingCallback};
    use crate::{sandbox_state::sandbox::EvolvableSandbox, MultiUseSandbox};
    use hyperlight_testing::{callback_guest_as_string, simple_guest_as_string};
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    // simple function
    fn test_function0(_: MultiUseGuestCallContext) -> Result<i32> {
        Ok(42)
    }

    struct GuestStruct;

    // function that return type unsupported by the host
    fn test_function1(_: SingleUseGuestCallContext) -> Result<GuestStruct> {
        Ok(GuestStruct)
    }

    // function that takes a parameter
    fn test_function2(_: MultiUseGuestCallContext, param: i32) -> Result<i32> {
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
                GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing")),
                None,
                None,
                None,
            )
            .unwrap()
        };

        // test_function0
        {
            let usbox = uninitialized_sandbox();
            let sandbox: MultiUseSandbox<'_> = usbox
                .evolve(MutatingCallback::from(init))
                .expect("Failed to initialize sandbox");
            let result = test_function0(sandbox.new_call_context());
            assert_eq!(result.unwrap(), 42);
        }

        // test_function1
        {
            let usbox = uninitialized_sandbox();
            let sandbox: SingleUseSandbox<'_> = usbox
                .evolve(MutatingCallback::from(init))
                .expect("Failed to initialize sandbox");
            let result = test_function1(sandbox.new_call_context());
            assert!(result.is_ok());
        }

        // test_function2
        {
            let usbox = uninitialized_sandbox();
            let sandbox: MultiUseSandbox<'_> = usbox
                .evolve(MutatingCallback::from(init))
                .expect("Failed to initialize sandbox");
            let result = test_function2(sandbox.new_call_context(), 42);
            assert_eq!(result.unwrap(), 42);
        }

        // test concurrent calls with a local closure that returns current count
        {
            let count = Arc::new(Mutex::new(0));
            let order = Arc::new(Mutex::new(vec![]));

            let mut handles = vec![];

            for _ in 0..10 {
                let usbox = uninitialized_sandbox();
                let sandbox: MultiUseSandbox = usbox
                    .evolve(MutatingCallback::from(init))
                    .expect("Failed to initialize sandbox");
                let _ctx = sandbox.new_call_context();
                let count = Arc::clone(&count);
                let order = Arc::clone(&order);
                let handle = thread::spawn(move || {
                    // we're not actually using the context, but we're calling
                    // it here to test the mutual exclusion
                    let mut num = count.lock().unwrap();
                    *num += 1;
                    order.lock().unwrap().push(*num);
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

    #[track_caller]
    fn guest_bin() -> GuestBinary {
        GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing"))
    }

    #[track_caller]
    fn test_call_guest_function_by_name(u_sbox: UninitializedSandbox<'_>) {
        let mu_sbox: MultiUseSandbox<'_> = u_sbox.evolve(MutatingCallback::from(init)).unwrap();

        let msg = "Hello, World!!\n".to_string();
        let len = msg.len() as i32;
        let mut ctx = mu_sbox.new_call_context();
        let result = ctx
            .call(
                "PrintOutput",
                ReturnType::Int,
                Some(vec![ParameterValue::String(msg.clone())]),
            )
            .unwrap();

        assert_eq!(result, ReturnValue::Int(len));
    }

    fn call_guest_function_by_name_hv() {
        // in-hypervisor mode
        let u_sbox = UninitializedSandbox::new(
            guest_bin(),
            // for now, we're using defaults. In the future, we should get
            // variability below
            None,
            // by default, the below represents in-hypervisor mode
            None,
            // just use the built-in host print function
            None,
        )
        .unwrap();
        test_call_guest_function_by_name(u_sbox);
    }

    #[test]
    fn test_call_guest_function_by_name_hv() {
        call_guest_function_by_name_hv();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_call_guest_function_by_name_in_proc_load_lib() {
        let u_sbox = UninitializedSandbox::new(
            guest_bin(),
            None,
            Some(crate::SandboxRunOptions::RunInProcess(true)),
            None,
        )
        .unwrap();
        test_call_guest_function_by_name(u_sbox);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_call_guest_function_by_name_in_proc_manual() {
        let u_sbox = UninitializedSandbox::new(
            guest_bin(),
            None,
            Some(crate::SandboxRunOptions::RunInProcess(false)),
            None,
        )
        .unwrap();
        test_call_guest_function_by_name(u_sbox);
    }

    fn terminate_vcpu_after_1000ms() -> Result<()> {
        // This test relies upon a Hypervisor being present so for now
        // we will skip it if there isnt one.
        if !is_hypervisor_present() {
            println!("Skipping test_call_guest_function_by_name because no hypervisor is present");
            return Ok(());
        }
        let usbox = UninitializedSandbox::new(
            GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing")),
            None,
            None,
            None,
        )?;
        let sandbox: MultiUseSandbox = usbox.evolve(MutatingCallback::from(init))?;
        let mut ctx = sandbox.new_call_context();
        let result = ctx.call("Spin", ReturnType::Void, None);

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

    // Test that we can terminate a VCPU that has been running the VCPU for too long.
    #[test]
    fn test_terminate_vcpu_spinning_cpu() -> Result<()> {
        terminate_vcpu_after_1000ms()?;
        Ok(())
    }

    // Because the terminate logic uses TLS to store state we need to ensure that once we have called terminate
    // on a thread we can create and initialize a new sandbox on that thread and it does not error
    #[test]
    fn test_terminate_vcpu_and_then_call_guest_function_on_the_same_host_thread() -> Result<()> {
        terminate_vcpu_after_1000ms()?;
        call_guest_function_by_name_hv();
        Ok(())
    }

    // This test is to capture the case where the guest execution is running a host function when cancelled and that host function
    // is never going to return.
    // The host function that is called will end after 5 seconds, but by this time the cancellation will have given up
    // (using default timeout settings)  , so this tests looks for the error "Failed to cancel guest execution".
    // Eventually once we fix https://github.com/deislabs/hyperlight/issues/951 this test should be updated.

    #[test]
    fn test_terminate_vcpu_calling_host_spinning_cpu() {
        // This test relies upon a Hypervisor being present so for now
        // we will skip it if there isnt one.
        if !is_hypervisor_present() {
            println!("Skipping test_call_guest_function_by_name because no hypervisor is present");
            return;
        }
        let mut usbox = UninitializedSandbox::new(
            GuestBinary::FilePath(callback_guest_as_string().expect("Guest Binary Missing")),
            None,
            None,
            None,
        )
        .unwrap();

        // Make this host call run for 5 seconds

        fn spin() -> Result<()> {
            thread::sleep(std::time::Duration::from_secs(5));
            Ok(())
        }

        let host_spin_func = Arc::new(Mutex::new(spin));

        host_spin_func.register(&mut usbox, "Spin").unwrap();

        let sandbox: MultiUseSandbox = usbox.evolve(MutatingCallback::from(init)).unwrap();
        let mut ctx = sandbox.new_call_context();
        let result = ctx.call("CallHostSpin", ReturnType::Void, None);

        assert!(result.is_err());
        match result.unwrap_err() {
            HyperlightError::HostFailedToCancelGuestExecution() => {}
            #[cfg(target_os = "linux")]
            HyperlightError::HostFailedToCancelGuestExecutionSendingSignals(_) => {}
            e => panic!("Unexpected Error got {:?}", e),
        }
    }
}
