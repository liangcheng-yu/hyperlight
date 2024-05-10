use std::sync::{Arc, Mutex};

use hyperlight_host::{
    func::{ParameterValue, ReturnType, ReturnValue},
    sandbox::SandboxConfiguration,
    GuestBinary, HyperlightError, UninitializedSandbox,
};
use hyperlight_testing::simple_guest_as_string;
use serial_test::serial; // using LoadLibrary requires serial tests

pub mod common; // pub to disable dead_code warning
use crate::common::get_sandboxes;

#[test]
#[serial]
fn pass_byte_array() {
    for sandbox in get_sandboxes(None).into_iter() {
        let mut ctx = sandbox.new_call_context();
        const LEN: usize = 10;
        let bytes = vec![1u8; LEN];
        let res = ctx.call(
            "SetByteArrayToZero",
            ReturnType::VecBytes,
            Some(vec![
                ParameterValue::VecBytes(bytes.clone()),
                ParameterValue::Int(LEN.try_into().unwrap()),
            ]),
        );

        match res.unwrap() {
            ReturnValue::VecBytes(res_bytes) => {
                assert_eq!(res_bytes.len(), LEN);
                assert!(res_bytes.iter().all(|&b| b == 0));
            }
            _ => panic!("Expected VecBytes"),
        }

        let res = ctx.call(
            "SetByteArrayToZeroNoLength",
            ReturnType::Int,
            Some(vec![ParameterValue::VecBytes(bytes.clone())]),
        );
        assert!(res.is_err()); // missing length param
    }
}

#[test]
#[serial]
fn invalid_guest_function_name() {
    for mut sandbox in get_sandboxes(None).into_iter() {
        let fn_name = "FunctionDoesntExist";
        let res = sandbox.call_guest_function_by_name(fn_name, ReturnType::Int, None);
        println!("{:?}", res);
        assert!(
            matches!(res.unwrap_err(), HyperlightError::GuestError(hyperlight_common::flatbuffer_wrappers::guest_error::ErrorCode::GuestFunctionNotFound, error_name) if error_name == fn_name)
        );
    }
}

#[test]
#[serial]
fn multiple_parameters() {
    let mut msgs = Vec::new();
    let writer = |msg| {
        msgs.push(msg);
        Ok(0)
    };

    let writer_func = Arc::new(Mutex::new(writer));

    let test_cases = vec![
        (
            "PrintTwoArgs",
            vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
            ],
            format!("Message: arg1:{} arg2:{}.", "1", 2),
        ),
        (
            "PrintThreeArgs",
            vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
                ParameterValue::Long(3),
            ],
            format!("Message: arg1:{} arg2:{} arg3:{}.", "1", 2, 3),
        ),
        (
            "PrintFourArgs",
            vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
                ParameterValue::Long(3),
                ParameterValue::String("4".to_string()),
            ],
            format!("Message: arg1:{} arg2:{} arg3:{} arg4:{}.", "1", 2, 3, "4"),
        ),
        (
            "PrintFiveArgs",
            vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
                ParameterValue::Long(3),
                ParameterValue::String("4".to_string()),
                ParameterValue::String("5".to_string()),
            ],
            format!(
                "Message: arg1:{} arg2:{} arg3:{} arg4:{} arg5:{}.",
                "1", 2, 3, "4", "5"
            ),
        ),
        (
            "PrintSixArgs",
            vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
                ParameterValue::Long(3),
                ParameterValue::String("4".to_string()),
                ParameterValue::String("5".to_string()),
                ParameterValue::Bool(true),
            ],
            format!(
                "Message: arg1:{} arg2:{} arg3:{} arg4:{} arg5:{} arg6:{}.",
                "1", 2, 3, "4", "5", true
            ),
        ),
        (
            "PrintSevenArgs",
            vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
                ParameterValue::Long(3),
                ParameterValue::String("4".to_string()),
                ParameterValue::String("5".to_string()),
                ParameterValue::Bool(true),
                ParameterValue::Bool(false),
            ],
            format!(
                "Message: arg1:{} arg2:{} arg3:{} arg4:{} arg5:{} arg6:{} arg7:{}.",
                "1", 2, 3, "4", "5", true, false
            ),
        ),
        (
            "PrintEightArgs",
            vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
                ParameterValue::Long(3),
                ParameterValue::String("4".to_string()),
                ParameterValue::String("5".to_string()),
                ParameterValue::Bool(true),
                ParameterValue::Bool(false),
                ParameterValue::String("8".to_string()),
            ],
            format!(
                "Message: arg1:{} arg2:{} arg3:{} arg4:{} arg5:{} arg6:{} arg7:{} arg8:{}.",
                "1", 2, 3, "4", "5", true, false, "8"
            ),
        ),
        (
            "PrintNineArgs",
            vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
                ParameterValue::Long(3),
                ParameterValue::String("4".to_string()),
                ParameterValue::String("5".to_string()),
                ParameterValue::Bool(true),
                ParameterValue::Bool(false),
                ParameterValue::String("8".to_string()),
                ParameterValue::Long(9),
            ],
            format!(
                "Message: arg1:{} arg2:{} arg3:{} arg4:{} arg5:{} arg6:{} arg7:{} arg8:{} arg9:{}.",
                "1", 2, 3, "4", "5", true, false, "8", 9
            ),
        ),
        (
            "PrintTenArgs",
            vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
                ParameterValue::Long(3),
                ParameterValue::String("4".to_string()),
                ParameterValue::String("5".to_string()),
                ParameterValue::Bool(true),
                ParameterValue::Bool(false),
                ParameterValue::String("8".to_string()),
                ParameterValue::Long(9),
                ParameterValue::Int(10),
            ],
            format!(
                "Message: arg1:{} arg2:{} arg3:{} arg4:{} arg5:{} arg6:{} arg7:{} arg8:{} arg9:{} arg10:{}.",
                "1", 2, 3, "4", "5", true, false, "8", 9, "10"
            ),
        ),
    ];

    for mut sandbox in get_sandboxes(Some(&writer_func)).into_iter() {
        for (fn_name, args, _expected) in test_cases.clone().into_iter() {
            let res = sandbox.call_guest_function_by_name(fn_name, ReturnType::Int, Some(args));
            assert!(res.is_ok());
        }
    }
    msgs.into_iter()
        .zip(test_cases)
        .for_each(|(printed_msg, expected)| assert!(printed_msg == expected.2));
}

#[test]
#[serial]
fn incorrect_parameter_type() {
    for mut sandbox in get_sandboxes(None).into_iter() {
        let res = sandbox.call_guest_function_by_name(
            "Echo",
            ReturnType::Int,
            Some(vec![
                ParameterValue::Int(2), // should be string
            ]),
        );
        assert!(matches!(
            res.unwrap_err(),
            HyperlightError::GuestError(
                hyperlight_common::flatbuffer_wrappers::guest_error::ErrorCode::GuestFunctionParameterTypeMismatch,
                msg
            ) if msg == "Function Echo parameter 0."
        ));
    }
}

#[test]
#[serial]
fn incorrect_parameter_num() {
    for mut sandbox in get_sandboxes(None).into_iter() {
        let res = sandbox.call_guest_function_by_name(
            "Echo",
            ReturnType::Int,
            Some(vec![
                ParameterValue::String("1".to_string()),
                ParameterValue::Int(2),
            ]),
        );
        assert!(matches!(
            res.unwrap_err(),
            HyperlightError::GuestError(
                hyperlight_common::flatbuffer_wrappers::guest_error::ErrorCode::GuestFunctionIncorrecNoOfParameters,
                msg
            ) if msg == "Called function Echo with 2 parameters but it takes 1."
        ));
    }
}

#[test]
fn max_memory_sandbox() {
    let mut cfg = SandboxConfiguration::default();
    cfg.set_input_data_size(0x40000000);
    let a = UninitializedSandbox::new(
        GuestBinary::FilePath(simple_guest_as_string().unwrap()),
        Some(cfg),
        None,
        None,
    );

    assert!(matches!(
        a.unwrap_err(),
        HyperlightError::MemoryRequestTooBig(..)
    ));
}
