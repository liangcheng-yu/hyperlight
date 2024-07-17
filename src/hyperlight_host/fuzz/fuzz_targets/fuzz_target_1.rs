#![no_main]

use hyperlight_host::func::{ParameterValue, ReturnType, ReturnValue};
use hyperlight_host::sandbox::uninitialized::GuestBinary;
use hyperlight_host::sandbox_state::sandbox::EvolvableSandbox;
use hyperlight_host::sandbox_state::transition::MutatingCallback;
use hyperlight_host::{MultiUseSandbox, Result, UninitializedSandbox};
use hyperlight_testing::simple_guest_as_string;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let u_sbox = UninitializedSandbox::new(
        GuestBinary::FilePath(simple_guest_as_string().expect("Guest Binary Missing")),
        None,
        None,
        None,
    )
    .unwrap();

    let mu_sbox: MultiUseSandbox<'_> = u_sbox.evolve(MutatingCallback::from(init)).unwrap();

    let msg = String::from_utf8_lossy(data).to_string();
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
});

fn init(_: &mut UninitializedSandbox) -> Result<()> {
    Ok(())
}
