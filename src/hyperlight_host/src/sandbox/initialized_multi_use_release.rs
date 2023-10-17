use super::guest_mgr::GuestMgr;
use crate::MultiUseSandbox;
use std::sync::{Arc, Mutex};

/// `ShouldRelease` is an internal construct that represents a
/// port of try-finally logic in C#.
///
/// It implements `drop` and captures part of our state in
/// `call_guest_function`, to allow it to properly act
/// on it and do cleanup.
pub(super) struct ShouldRelease<'a>(bool, Arc<Mutex<MultiUseSandbox<'a>>>);

impl<'a> ShouldRelease<'a> {
    pub(super) fn new(val: bool, sbox_arc: Arc<Mutex<MultiUseSandbox<'a>>>) -> Self {
        Self(val, sbox_arc)
    }

    pub(super) fn toggle(&mut self) {
        self.0 = !self.0;
    }
}

impl<'a> Drop for ShouldRelease<'a> {
    fn drop(&mut self) {
        if self.0 {
            let sbox = &mut self.1.lock().unwrap();
            sbox.set_needs_state_reset(true);
            let executing_guest_function = sbox.get_executing_guest_call_mut();
            executing_guest_function.store(0);
        }
    }
}

pub(super) struct ShouldReset<'a>(bool, Arc<Mutex<MultiUseSandbox<'a>>>);

impl<'a> ShouldReset<'a> {
    pub(super) fn new(val: bool, mu_sbox: MultiUseSandbox<'a>) -> Self {
        Self(val, Arc::new(Mutex::new(mu_sbox)))
    }
}

impl<'a> Drop for ShouldReset<'a> {
    fn drop(&mut self) {
        let mut sbox = self.1.lock().unwrap();
        sbox.exit_method(self.0);
    }
}
