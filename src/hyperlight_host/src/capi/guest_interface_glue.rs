use crate::{capi::hdl::Hdl, guest_interface_glue::validate_type_supported};

use super::{c_func::CFunc, context::Context, handle::Handle};

/// DAN:TODO
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn guest_interface_glue_validate_type_supported(
    context: *mut Context,
    some_type: Handle,
) -> Handle {
    CFunc::new("guest_interface_glue_validate_type_supported", context)
        .and_then_mut(|ctx, _| {
            let some_type = Context::get(some_type, &ctx.strings, |h| {
                matches!(h, Hdl::String(_))
            })?;
            match validate_type_supported(some_type) {
                Ok(_) => Ok(Handle::new_empty()),
                Err(e) => Ok((*ctx).register_err(e)),
            }
        })
        .ok_or_err_hdl()
}
