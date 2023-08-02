pub trait ExposeFuncs {
    fn expose_and_bind_members(&mut self, _exposed_methods: proc_macro2::TokenStream) {
        // First, parse exposed_methods TokenStream to separate methods exposed to the guest, and host
        // (i.e., identified by #[expose_to(guest)], and #[expose_to(host)] respectively)

        // If a method is being exposed to the guest, we want to register it onto the Sandbox, similarly to how we've done
        // w/ writer_func and tests.
        // For example, if provided w/:
        //  fn host_method(a1: String) -> i32 {
        //      print_output(a1)
        //  }
        // We want to:
        // use crate::func::host::HostFunction1;
        // host_method.register(&mut sbox, "host_method");

        // If a method is being exposed to the host, we want to generate a closure for it, w/ appropriate wrapping.
        // For example:
        // fn guest_method(a1: String) -> i32;

        // We would generate:
        //  let guest_method = |a1: String| -> i32 {
        //      call_dynamic_guest_func("guest_method", vec![a1]);
        //      // ^^^ in itself, `call_dynamic_guest_func` will be #[instrument]ed, it will have a `try-finally`-like
        //      // logic to always call the correspondant `exit_dynamic_method(should_reset)` to the `enter_dynamic_method()`
        //      // call it makes. Other than that, if `should_reset`, it will call `reset_state()`, and, regardless, return a
        //      // dispatch_call_from_host("guest_method", ReturnType::Int ,vec![a1]);
        //  };
        // Like our host functions, these dynamic methods are also added to a HashMap.
    }
}
