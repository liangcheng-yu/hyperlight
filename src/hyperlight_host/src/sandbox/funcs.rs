#![allow(dead_code)]

#[allow(unused)]
use crate::func::host::{
    HostFunction0, HostFunction1, HostFunction10, HostFunction2, HostFunction3, HostFunction4,
    HostFunction5, HostFunction6, HostFunction7, HostFunction8, HostFunction9,
};

#[allow(unused)]
use crate::func::guest::{
    DynamicGuestFunction0, DynamicGuestFunction1, DynamicGuestFunction10, DynamicGuestFunction2,
    DynamicGuestFunction3, DynamicGuestFunction4, DynamicGuestFunction5, DynamicGuestFunction6,
    DynamicGuestFunction7, DynamicGuestFunction8, DynamicGuestFunction9,
};

use crate::{
    func::types::{ParameterType, ReturnType},
    sandbox_state::sandbox::UninitializedSandbox,
};

use super::guest_funcs::CallGuestFunction;

struct GuestMethod {
    name: String,
    args: Vec<ParameterType>,
    return_type: ReturnType,
}

impl GuestMethod {
    fn new(name: &str, args: Vec<ParameterType>, return_type: ReturnType) -> Self {
        Self {
            name: name.to_string(),
            args,
            return_type,
        }
    }
}

pub trait ExposeFuncs<'a>: UninitializedSandbox<'a> + CallGuestFunction<'a> {
    fn expose_and_bind_members(&mut self, _exposed_methods: proc_macro2::TokenStream) {
        // First, parse exposed_methods TokenStream to separate methods exposed to the guest, and host
        // (i.e., identified by #[expose_to(guest)], and #[expose_to(host)] respectively)
        // For example, if provided w/:
        // let exposed_methods = hyperlight_macro::expose_methods! {
        //     trait ExposedMethods: CallGuestFunction<'a> {
        //         #[expose_to(host)]
        //         fn guest_method(a1: String) -> i32;
        //
        //         #[expose_to(host)]
        //         fn print_output(a1: String) -> i32;
        //
        //         #[expose_to(guest)]
        //         fn host_method(&self, a1: String) -> Result<i32> {
        //             self.call_dynamic_guest_function("print_output", ReturnType::Int, vec![a1])
        //         }
        //     }
        //     }; // <-  this is of type proc_macro2::TokenStream

        // For guest methods, we want to generate:
        //  let guest_methods = vec![
        //      GuestMethod::new("guest_method", vec![ParameterType::String], ReturnType::Int),
        //      GuestMethod::new("print_output", vec![ParameterType::String], ReturnType::Int),
        //  ];
        // Then, for each of these, we should generate:
        //  let guest_method = |a1: String| -> i32 {
        //      self.call_dynamic_guest_function("guest_method", ReturnType::Int, vec![a1]);
        //      // ^^^ in itself, `create_and_dispatch_dynamic_function_guest_call` will be #[instrument]ed, it will have a `try-finally`-like
        //      // logic to always call the correspondant `exit_dynamic_method(should_reset)` to the `enter_dynamic_method()`
        //      // call it makes. Other than that, if `should_reset`, it will call `reset_state()`, and, regardless, return a
        //      // dispatch_call_from_host("guest_method", ReturnType::Int ,vec![a1]);
        //  };
        // guest_method.register(self.get_uninitialized_sandbox_mut(), "guest_method");

        // For host methods, we want to generate:
        // let host_method = |a1: String| -> i32 {
        //     self.call_dynamic_guest_function("print_output", ReturnType::Int, vec![a1])
        //     // ^^^ i.e., maintaining the function's original body.
        // };
        // host_method.register(self.get_uninitialized_sandbox_mut(), "host_method");
    }
}
