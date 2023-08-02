use std::sync::atomic::AtomicI32;

pub(crate) trait GuestMgr {
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
}
