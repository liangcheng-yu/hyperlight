use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemTrait};

/// # `expose_and_bind_members`
/// This function-like macro parses its input `TokenStream` to separate methods exposed to the guest, and host
/// (i.e., identified by #[expose_to(guest)], and #[expose_to(host)], respectively).
///
/// - For example, if provided w/ the following input:
/// ```
/// trait ExposedMethods<'a> {
///     #[expose_to(host)]
///     fn guest_method(a1: String) -> i32;
///
///     #[expose_to(host)]
///     fn print_output(a1: String) -> i32;
///
///     #[expose_to(guest)]
///     fn host_method(a1: String) -> i32 {
///         Self::print_output(a1)
///     }
/// }
/// ```
///
/// - For guest methods (i.e., decorated w/ `#[expose_to(host)]`), it'd generate:
///  let guest_method = |a1: String| -> i32 {
///      self.call_dynamic_guest_function("guest_method", ReturnType::Int, vec![a1]);
///  };
/// guest_method.register(self.get_uninitialized_sandbox_mut(), "guest_method");
///
/// - For host methods, it'd generate:
/// let
/// let host_method = |a1: String| -> i32 {
///     self.call_dynamic_guest_function("print_output", ReturnType::Int, vec![a1])
///     // ^^^ i.e., maintaining the function's original body.
/// };
/// host_method.register(self.get_uninitialized_sandbox_mut(), "host_method");
#[proc_macro]
pub fn expose_and_bind_members(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemTrait);

    quote! {
        #input
    }
    .into()
}

/// # `expose_to`
/// This attribute macro doesn't particularly do anything, but it's used to identify methods that are exposed to the guest
/// or host.
#[proc_macro_attribute]
pub fn expose_to(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
