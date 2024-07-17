use hyperlight_common::flatbuffer_wrappers::function_types::{ParameterValue, ReturnType};
use hyperlight_host::func::call_ctx::MultiUseGuestCallContext;
use hyperlight_host::sandbox::{MultiUseSandbox, UninitializedSandbox};
use hyperlight_host::sandbox_state::sandbox::EvolvableSandbox;
use hyperlight_host::sandbox_state::transition::Noop;
use hyperlight_host::{new_error, GuestBinary, Result};
use hyperlight_testing::simple_guest_as_string;

fn main() {
    // create a new `MultiUseSandbox` configured to run the `simpleguest.exe`
    // test guest binary
    let sbox1: MultiUseSandbox = {
        let path = simple_guest_as_string().unwrap();
        let u_sbox =
            UninitializedSandbox::new(GuestBinary::FilePath(path), None, None, None).unwrap();
        u_sbox.evolve(Noop::default())
    }
    .unwrap();

    // create a new call context from the sandbox, then do some calls with it.
    let ctx1 = sbox1.new_call_context();
    let sbox2 = do_calls(ctx1).unwrap();
    // create a new call context from the returned sandbox, then do some calls
    // with that one
    let ctx2 = sbox2.new_call_context();
    do_calls(ctx2).unwrap();
}

/// Given a `MultiUseGuestCallContext` derived from an existing
/// `MultiUseSandbox` configured to run the `simpleguest.exe` test guest
/// binary, do several calls against that binary, print their results, then
/// call `ctx.finish()` and return the resulting `MultiUseSandbox`. Return an `Err`
/// if anything failed.
fn do_calls(mut ctx: MultiUseGuestCallContext) -> Result<MultiUseSandbox> {
    {
        let res1: i32 = {
            let rv = ctx.call(
                "StackAllocate",
                ReturnType::Int,
                Some(vec![ParameterValue::Int(1)]),
            )?;
            rv.try_into()
        }
        .map_err(|e| new_error!("failed to get StackAllocate result: {}", e))?;
        println!("got StackAllocate res: {res1}");
    }
    {
        let res2: i32 = {
            let rv = ctx.call(
                "CallMalloc",
                ReturnType::Int,
                Some(vec![ParameterValue::Int(200)]),
            )?;
            rv.try_into()
        }
        .map_err(|e| new_error!("failed to get CallMalloc result: {}", e))?;
        println!("got CallMalloc res: {res2}");
    }
    ctx.finish()
}
