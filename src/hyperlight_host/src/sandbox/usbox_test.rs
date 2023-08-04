// #[allow(unused)]
// use crate::func::host::{
//     HostFunction0, HostFunction1, HostFunction10, HostFunction2, HostFunction3, HostFunction4,
//     HostFunction5, HostFunction6, HostFunction7, HostFunction8, HostFunction9,
// };

// #[allow(unused)]
// use crate::func::guest::{
//     DynamicGuestFunction0, DynamicGuestFunction1, DynamicGuestFunction10, DynamicGuestFunction2,
//     DynamicGuestFunction3, DynamicGuestFunction4, DynamicGuestFunction5, DynamicGuestFunction6,
//     DynamicGuestFunction7, DynamicGuestFunction8, DynamicGuestFunction9,
// };

use hyperlight_macro::expose_to;
use crate::{UninitializedSandbox, func::types::ReturnType, Sandbox};

use super::guest_funcs::CallGuestFunction;

trait ExposedMethods<'a> {
    #[expose_to(host)]
    fn guest_method(sbox: &mut Sandbox<'a>, a1: String) -> i32;

    #[expose_to(host)]
    fn print_output(sbox: &mut Sandbox<'a>, a1: String) -> i32;

    #[expose_to(guest)]
    fn host_method(sbox: &mut Sandbox<'a>, a1: String) -> i32 {
        Self::print_output(sbox, a1)
    }
}

impl<'a> ExposedMethods<'a> for UninitializedSandbox<'a> {
    fn guest_method(sbox: &mut Sandbox<'a>, a1: String) -> i32 {
        sbox.call_dynamic_guest_function("guest_method", ReturnType::Int, Some(vec![a1])).expect("failed to call guest function")
    }

    fn print_output(sbox: &mut Sandbox<'a>, a1: String) -> i32 {
        sbox.call_dynamic_guest_function("print_output", ReturnType::Int, Some(vec![a1])).expect("failed to call guest function")
    }

    fn host_method(sbox: &mut Sandbox<'a>, a1: String) -> i32 {
        Self::print_output(sbox, a1)
    }
}