use super::transition::TransitionMetadata;
use crate::sandbox::hypervisor::HypervisorWrapper;
use crate::Result;
use std::thread::JoinHandle;
use std::{fmt::Debug, panic};
use tracing::{instrument, Span};

/// The minimal functionality of a Hyperlight sandbox. Most of the types
/// and operations within this crate require `Sandbox` implementations.
///
/// `Sandbox`es include the notion of an ordering in a state machine.
/// For example, a given `Sandbox` implementation may be the root node
/// in the state machine to which it belongs, and it may know how to "evolve"
/// into a next state. That "next state" may in turn know how to roll back
/// to the root node.
///
/// These transitions are expressed as `EvolvableSandbox` and
/// `DevolvableSandbox` implementations any `Sandbox` implementation can
/// opt into.
pub trait Sandbox: Sized + Debug {
    /// Check to ensure the current stack cookie matches the one that
    /// was selected when the stack was constructed.
    ///
    /// Return an `Err` if there was an error inspecting the stack, `Ok(false)`
    /// if there was no such error but the stack guard doesn't match, and
    /// `Ok(true)` in the same situation where the stack guard does match.
    ///

    // NOTE: this is only needed for the C API and for UnitilizedSandbox, SingleUseSandbox, and MultiUseSandbox
    // Those are the only types that need implement this trait
    // The default implementation is provided so that types that implement Sandbox (e.g. JSSandbox) but do not need to implement this trait do not need to provide an implementation
    // TODO: Once the C API has been updated to use the Rust API then we can remove this
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn check_stack_guard(&self) -> Result<bool> {
        panic!("check_stack_guard not implemented for this type");
    }

    /// Every `Sandbox` `impl`ementor (i.e., `SingleUseSandbox`, and `MultiUseSandbox` has a
    /// `HypervisorWrapper` field. This method allows you to get a reference to that field.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_hypervisor_wrapper_mut(&mut self) -> &mut HypervisorWrapper {
        panic!("get_hypervisor_wrapper_mut not implemented for this type");
    }

    /// Every `Sandbox` `impl`ementor (i.e., `SingleUseSandbox`, and `MultiUseSandbox` has a
    /// `JoinHandle` field, due to its associated Hypervisor Handler Thread. This method
    /// allows you to get a reference to that field.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn get_hypervisor_handler_thread_mut(&mut self) -> &mut Option<JoinHandle<Result<()>>> {
        panic!("get_hypervisor_handler_thread_mut not implemented for this type");
    }
}

/// A utility trait to recognize a Sandbox that has not yet been initialized.
/// It allows retrieval of a strongly typed UninitializedSandbox.
pub trait UninitializedSandbox<'a>: Sandbox {
    fn get_uninitialized_sandbox(&self) -> &crate::sandbox::UninitializedSandbox;

    fn get_uninitialized_sandbox_mut(&mut self) -> &mut crate::sandbox::UninitializedSandbox;

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn is_running_in_process(&self) -> bool {
        self.get_uninitialized_sandbox().run_from_process_memory
    }
}

/// A `Sandbox` that knows how to "evolve" into a next state.
pub trait EvolvableSandbox<Cur: Sandbox, Next: Sandbox, T: TransitionMetadata<Cur, Next>>:
    Sandbox
{
    fn evolve(self, tsn: T) -> Result<Next>;
}

/// A `Sandbox` that knows how to roll back to a "previous" `Sandbox`
pub trait DevolvableSandbox<Cur: Sandbox, Prev: Sandbox, T: TransitionMetadata<Cur, Prev>>:
    Sandbox
{
    fn devolve(self, tsn: T) -> Result<Prev>;
}
