use hyperlight_flatbuffers::flatbuffer_wrappers::guest_error::ErrorCode;
use hyperlight_host::func::{ParameterValue, ReturnType, ReturnValue};
use hyperlight_host::sandbox_state::sandbox::EvolvableSandbox;
use hyperlight_host::sandbox_state::transition::Noop;
use hyperlight_host::{GuestBinary, HyperlightError, Result};
use hyperlight_host::{SingleUseSandbox, UninitializedSandbox};
use hyperlight_testing::{c_simple_guest_as_string, simple_guest_as_string};
use strum::IntoEnumIterator;

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
    assert!(
        matches!(res, HyperlightError::GuestAborted(code, message) if (code == error_code && message.is_empty()) )
    );
}

#[test]
fn guest_abort_with_context1() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let res = ctx1
        .call(
            "abort_with_code_and_message",
            ReturnType::Void,
            Some(vec![
                ParameterValue::Int(25),
                ParameterValue::String("Oh no".to_string()),
            ]),
        )
        .unwrap_err();
    println!("{:?}", res);
    assert!(
        matches!(res, HyperlightError::GuestAborted(code, context) if (code == 25 && context == "Oh no"))
    );
}

#[test]
fn guest_abort_with_context2() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    // The buffer size for the panic context is 1024 bytes.
    // This test will see what happens if the panic message is longer than that
    let abort_message = "Lorem ipsum dolor sit amet, \
                                consectetur adipiscing elit, \
                                sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                                Nec feugiat nisl pretium fusce. \
                                Amet mattis vulputate enim nulla aliquet porttitor lacus. \
                                Nunc congue nisi vitae suscipit tellus. \
                                Erat imperdiet sed euismod nisi porta lorem mollis aliquam ut. \
                                Amet tellus cras adipiscing enim eu turpis egestas. \
                                Blandit volutpat maecenas volutpat blandit aliquam etiam erat velit scelerisque. \
                                Tristique senectus et netus et malesuada. \
                                Eu turpis egestas pretium aenean pharetra magna ac placerat vestibulum. \
                                Adipiscing at in tellus integer feugiat. \
                                Faucibus vitae aliquet nec ullamcorper sit amet risus. \
                                \n\
                                Eros in cursus turpis massa tincidunt dui. \
                                Purus non enim praesent elementum facilisis leo vel fringilla. \
                                Dolor sit amet consectetur adipiscing elit pellentesque habitant morbi. \
                                Id leo in vitae turpis. At lectus urna duis convallis convallis tellus id interdum. \
                                Purus sit amet volutpat consequat. Egestas purus viverra accumsan in. \
                                Sodales ut etiam sit amet nisl. Lacus sed viverra tellus in hac. \
                                Nec ullamcorper sit amet risus nullam eget. \
                                Adipiscing bibendum est ultricies integer quis auctor. \
                                Vitae elementum curabitur vitae nunc sed velit dignissim sodales ut. \
                                Auctor neque vitae tempus quam pellentesque nec. \
                                Non pulvinar neque laoreet suspendisse interdum consectetur libero. \
                                Mollis nunc sed id semper. \
                                Et sollicitudin ac orci phasellus egestas tellus rutrum tellus pellentesque. \
                                Arcu felis bibendum ut tristique et. \
                                Proin sagittis nisl rhoncus mattis rhoncus urna. Magna eget est lorem ipsum.";

    let res = ctx1
        .call(
            "abort_with_code_and_message",
            ReturnType::Void,
            Some(vec![
                ParameterValue::Int(60),
                ParameterValue::String(abort_message.to_string()),
            ]),
        )
        .unwrap_err();
    println!("{:?}", res);
    assert!(
        matches!(res, HyperlightError::GuestAborted(_, context) if context.contains(&abort_message[..400]))
    );
}

// Ensure abort with context works for c guests.
// Just run this manually for now since we only build c guests on Windows and will
// hopefully be removing the c guest library soon.
#[test]
#[ignore]
fn guest_abort_c_guest() {
    let path = c_simple_guest_as_string().unwrap();
    let guest_path = GuestBinary::FilePath(path);
    let uninit = UninitializedSandbox::new(guest_path, None, None, None);
    let sbox1: SingleUseSandbox = uninit.unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let res = ctx1
        .call(
            "GuestAbortWithMessage",
            ReturnType::Void,
            Some(vec![
                ParameterValue::Int(75_i32),
                ParameterValue::String("This is a test error message".to_string()),
            ]),
        )
        .unwrap_err();
    println!("{:?}", res);
    assert!(
        matches!(res, HyperlightError::GuestAborted(code, message) if (code == 75 && message == "This is a test error message"))
    );
}

#[test]
fn guest_panic() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let res = ctx1
        .call(
            "guest_panic",
            ReturnType::Void,
            Some(vec![ParameterValue::String(
                "Error... error...".to_string(),
            )]),
        )
        .unwrap_err();
    println!("{:?}", res);
    assert!(
        matches!(res, HyperlightError::GuestAborted(code, context) if code == 0 && context.contains("\nError... error..."))
    )
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
        matches!(res, HyperlightError::GuestAborted(code, _) if code == ErrorCode::MallocFailed as u8)
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

// checks alloca fails with stackoverflow for large allocations
#[test]
fn dynamic_stack_allocate_overflow() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    // zero is handled as special case in guest,
    // will turn DEFAULT_GUEST_STACK_SIZE + 1
    let bytes = 0;

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

// checks alloca fails with overflow when stack pointer overflows
#[test]
fn dynamic_stack_allocate_pointer_overflow() {
    let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
    let mut ctx1 = sbox1.new_call_context();

    let bytes = 10 * 1024 * 1024; // 10Mb

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

// checks alloca fails with stackoverflow for huge allocations with c guest lib
#[test]
#[ignore]
fn dynamic_stack_allocate_overflow_c_guest() {
    let path = c_simple_guest_as_string().unwrap();
    let guest_path = GuestBinary::FilePath(path);
    let uninit = UninitializedSandbox::new(guest_path, None, None, None);
    let sbox1: SingleUseSandbox = uninit.unwrap().evolve(Noop::default()).unwrap();
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

// Check that log messages are emitted correctly from the guest
// This test is ignored as it sets a logger and therefore maybe impacted by other tests running concurrently
// or it may impact other tests.
// It will run from the command just test-rust as it is included in that target
// It can also be run explicitly with `cargo test --test integration_test log_message -- --ignored`
#[test]
#[ignore]
fn log_message() {
    use hyperlight_testing::{simplelogger::SimpleLogger, simplelogger::LOGGER};
    SimpleLogger::initialize_test_logger();
    LOGGER.set_max_level(log::LevelFilter::Trace);
    // The hyperlight guest should derive its max log level from the level set here for the host
    // this then should restrict the number of log messages that are emitted by the guest
    // so that they do not have to be filtered out by the host.
    log::set_max_level(log::LevelFilter::Trace);
    LOGGER.clear_log_calls();
    assert_eq!(0, LOGGER.num_log_calls());

    log_test_messages();
    assert_eq!(6, LOGGER.num_log_calls());

    log::set_max_level(log::LevelFilter::Error);
    LOGGER.set_max_level(log::LevelFilter::Error);
    LOGGER.clear_log_calls();
    assert_eq!(0, LOGGER.num_log_calls());

    log_test_messages();
    assert_eq!(2, LOGGER.num_log_calls());
    // The number of enabled calls is the number of times that the enabled function is called
    // with a target of "hyperlight_guest"
    // This should be the same as the number of log calls as all the log calls for the "hyperlight_guest" target should be filtered in
    // the guest
    assert_eq!(LOGGER.num_log_calls(), LOGGER.num_enabled_calls());
}

fn log_test_messages() {
    for level in hyperlight_flatbuffers::flatbuffer_wrappers::guest_log_level::LogLevel::iter() {
        if level == hyperlight_flatbuffers::flatbuffer_wrappers::guest_log_level::LogLevel::None {
            continue;
        }
        let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
        let mut ctx1 = sbox1.new_call_context();

        let message = format!("Hello from log_message level {}", level as i32);
        ctx1.call(
            "LogMessage",
            ReturnType::Void,
            Some(vec![
                ParameterValue::String(message.to_string()),
                ParameterValue::Int(level as i32),
            ]),
        )
        .unwrap();
    }
}
