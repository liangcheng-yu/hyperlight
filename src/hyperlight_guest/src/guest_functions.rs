use alloc::vec::Vec;
use hyperlight_common::flatbuffer_wrappers::guest_function_definition::GuestFunctionDefinition;

use crate::{GUEST_FUNCTIONS, GUEST_FUNCTIONS_BUILDER};

pub fn register_function(function_definition: GuestFunctionDefinition) {
    unsafe {
        // This is currently safe, because we are single threaded, but we
        // should find a better way to do this, see issue #808
        #[allow(static_mut_refs)]
        let gfd = &mut GUEST_FUNCTIONS_BUILDER;
        gfd.push(function_definition);
    }
}

pub fn finalise_function_table() {
    unsafe {
        // This is currently safe, because we are single threaded, but we
        // should find a better way to do this, see issue #808
        #[allow(static_mut_refs)]
        let gfd = &mut GUEST_FUNCTIONS_BUILDER;
        gfd.sort_by_function_name();

        let gfd_finalised: Vec<u8> =
            Vec::try_from(&*gfd).expect("Could not convert GUEST_FUNCTIONS_BUILDER to Vec<u8>");
        GUEST_FUNCTIONS = gfd_finalised.clone();
    }
}
