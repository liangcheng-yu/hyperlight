use hyperlight_flatbuffers::flatbuffer_wrappers::guest_error::ErrorCode;
use hyperlight_host::func::{ParameterValue, ReturnType};
use hyperlight_host::sandbox_state::sandbox::EvolvableSandbox;
use hyperlight_host::sandbox_state::transition::Noop;
use hyperlight_host::{GuestBinary, HyperlightError, Result};
use hyperlight_host::{SingleUseSandbox, UninitializedSandbox};
use hyperlight_testing::simple_guest_as_string;

fn new_uninit<'a>() -> Result<UninitializedSandbox<'a>> {
    let path = simple_guest_as_string().unwrap();
    UninitializedSandbox::new(
        GuestBinary::FilePath(path),
        None,
        None, //Some(hyperlight_host::SandboxRunOptions::RunInProcess(true)),
        None,
    )
}

// Makes sure that the guest can abort with a specific code.
// Note that this tests will fail if hloutb is optimized away
// which can happen, see #1141
#[test]
fn test_guest_abort() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let error_code: u8 = 13; // this is arbitrary
    let res = ctx1
        .call(
            "test_abort",
            ReturnType::Void,
            Some(vec![ParameterValue::Int(error_code as i32)]),
        )
        .unwrap_err();
    println!("{:?}", res);
    assert!(matches!(res, HyperlightError::GuestAborted(code) if code == error_code));
}

#[test]
fn test_guest_malloc_abort() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let size = 20000000; // some big number that should fail when allocated
    let res = ctx1
        .call(
            "test_rust_malloc",
            ReturnType::Int,
            Some(vec![ParameterValue::Int(size)]),
        )
        .unwrap_err();
    println!("{:?}", res);
    assert!(
        matches!(res, HyperlightError::GuestAborted(code) if code == ErrorCode::MallocFailed as u8)
    );
}

#[test]
fn test_guest_malloc() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let size = 200; // some small number that should be ok
    let _res = ctx1
        .call(
            "test_rust_malloc",
            ReturnType::Int,
            Some(vec![ParameterValue::Int(size)]),
        )
        .unwrap();
}
