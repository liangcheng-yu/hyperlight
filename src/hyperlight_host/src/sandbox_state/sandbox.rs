use super::transition::TransitionMetadata;
use crate::Result;
use std::fmt::Debug;
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
    /// By default, a Sandbox is non-reusable
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn is_reusable(&self) -> bool {
        false
    }

    /// Check to ensure the current stack cookie matches the one that
    /// was selected when the stack was constructed.
    ///
    /// Return an `Err` if there was an error inspecting the stack, `Ok(false)`
    /// if there was no such error but the stack guard doesn't match, and
    /// `Ok(true)` in the same situation where the stack guard does match.
    fn check_stack_guard(&self) -> Result<bool>;
}

/// A utility trait to recognize a Sandbox that has not yet been initialized.
/// It allows retrieval of a strongly typed UninitializedSandbox.
pub trait UninitializedSandbox<'a>: Sandbox {
    fn get_uninitialized_sandbox(&self) -> &crate::sandbox::UninitializedSandbox<'a>;

    fn get_uninitialized_sandbox_mut(&mut self) -> &mut crate::sandbox::UninitializedSandbox<'a>;

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
