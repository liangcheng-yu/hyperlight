use crate::{capi::hdl::Hdl, guest_interface_glue::validate_type_supported};

use super::{c_func::CFunc, context::Context, handle::Handle};

/// Validates that the given type is supported by the host interface.
/// 
/// # Safety
/// - This function expects to receive a Handle to a String as the second argument.
/// - If type is not supported, it will yield an error Handle.
#[no_mangle]
pub unsafe extern "C" fn guest_interface_glue_validate_type_supported(
    context: *mut Context,
    some_type: Handle,
) -> Handle {
    CFunc::new("guest_interface_glue_validate_type_supported", context)
        .and_then_mut(|ctx, _| {
            let some_type = Context::get(some_type, &ctx.strings, |h| matches!(h, Hdl::String(_)))?;
            validate_type_supported(some_type).map(|_| Handle::new_empty())
        })
        .ok_or_err_hdl()
}
