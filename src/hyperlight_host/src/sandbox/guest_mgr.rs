use log::error;
use std::sync::atomic::{AtomicI32, Ordering};

pub trait GuestMgr {
    /// Get an immutable reference to the internally-stored
    /// `executing_guest_call` flag
    fn get_executing_guest_call(&self) -> &AtomicI32;

    /// Get a mutable reference to the internally-stored
    /// `executing_guest_call` flag
    fn get_executing_guest_call_mut(&mut self) -> &mut AtomicI32;

    /// Increase the number of times guest funcs have been called
    fn increase_num_runs(&mut self);

    /// Get the number of times guest funcs have been called
    fn get_num_runs(&self) -> i32;

    /// Checks if the `Sandbox` needs state resetting.
    fn needs_state_reset(&self) -> bool;

    /// Sets the `Sandbox`'s `needs_state_reset` property to provided value.
    fn set_needs_state_reset(&mut self, val: bool);

    /// Get immutable reference as `Box<dyn GuestMgr>`
    fn as_guest_mgr(&self) -> &dyn GuestMgr;

    /// Get mutable reference as `Box<dyn GuestMgr>`
    fn as_guest_mgr_mut(&mut self) -> &mut dyn GuestMgr;

    /// `enter_dynamic_method` is used to indicate if a `Sandbox`'s state should be reset.
    /// - When we enter call a guest function, the `executing_guest_call` value is set to 1.
    /// - When we exit a guest function, the `executing_guest_call` value is set to 0.
    ///
    /// `enter_dynamic_method` will check if the value of `executing_guest_call` is 1.
    /// If yes, it means the guest function is still running and state should not be reset.
    /// If the value of `executing_guest_call` is 0, we should reset the state.
    fn enter_dynamic_method(&mut self) -> bool {
        let executing_guest_function = self.get_executing_guest_call_mut();
        if executing_guest_function.load(Ordering::SeqCst) == 1 {
            return false;
        }

        if executing_guest_function
            .compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst)
            .unwrap() // .compare_exchange() returns a Result<i32, i32>, so it's ok to unwrap here
            != 0
        {
            error!("Guest call already in progress");
        }

        true
    }

    /// `exit_dynamic_method` is used to indicate that a guest function has finished executing.
    fn exit_dynamic_method(&mut self, should_release: bool) {
        if should_release {
            self.get_executing_guest_call_mut()
                .store(0, Ordering::SeqCst);
            self.set_needs_state_reset(true);
        }
    }
}
