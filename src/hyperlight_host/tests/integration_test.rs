use hyperlight_flatbuffers::flatbuffer_wrappers::guest_error::ErrorCode;
use hyperlight_host::func::{ParameterValue, ReturnType, ReturnValue};
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

// Checks that guest can abort with a specific code.
#[test]
fn guest_abort() {
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

// checks that malloc works
#[test]
fn guest_malloc() {
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

// checks that malloc failures are captured correctly
#[test]
fn guest_malloc_abort() {
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

// checks that alloca works
#[test]
fn dynamic_stack_allocate() {
    let sbox: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx = sbox.new_call_context();

    let bytes = 10_000; // some low number that can be allocated on stack

    ctx.call(
        "StackAllocate",
        ReturnType::Int,
        Some(vec![ParameterValue::Int(bytes)]),
    )
    .unwrap();
}

// checks alloca fails with stackoverflow for huge allocations
#[test]
fn dynamic_stack_allocate_overflow() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let bytes = 0; // zero is handled as special case in guest, will turn into large number

    let res = ctx1
        .call(
            "StackAllocate",
            ReturnType::Int,
            Some(vec![ParameterValue::Int(bytes)]),
        )
        .unwrap_err();
    println!("{:?}", res);
    assert!(matches!(res, HyperlightError::StackOverflow()));
}

// checks that a small buffer on stack works
#[test]
fn static_stack_allocate() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let res = ctx1
        .call("SmallVar", ReturnType::Int, Some(Vec::new()))
        .unwrap();
    assert!(matches!(res, ReturnValue::Int(1024)));
}

// checks that a huge buffer on stack fails with stackoverflow
#[test]
fn static_stack_allocate_overflow() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();
    let res = ctx1
        .call("LargeVar", ReturnType::Int, Some(Vec::new()))
        .unwrap_err();
    assert!(matches!(res, HyperlightError::StackOverflow()));
}

// checks that a recursive function with stack allocation works
#[test]
fn recursive_stack_allocate() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let iterations = 1;

    ctx1.call(
        "StackOverflow",
        ReturnType::Int,
        Some(vec![ParameterValue::Int(iterations)]),
    )
    .unwrap();
}

// checks that a recursive function with stack allocation eventually fails with stackoverflow
#[test]
fn recursive_stack_allocate_overflow() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let iterations = 10;

    let res = ctx1
        .call(
            "StackOverflow",
            ReturnType::Void,
            Some(vec![ParameterValue::Int(iterations)]),
        )
        .unwrap_err();
    assert!(matches!(res, HyperlightError::StackOverflow()));
}
