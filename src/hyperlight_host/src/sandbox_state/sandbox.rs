use super::transition::TransitionMetadata;
use anyhow::Result;
use std::fmt::Debug;

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
pub trait Sandbox: Sized + Debug {}

/// A "final" sandbox implementation that has the following properties:
///
/// - Can execute guest code, potentially more than once
/// - Can be devolved, but not evolved
///
/// These properties imply the following about `ReusablsSandbox`:
///
/// - Its `run` method borrows `&self` (rather than consuming it), so callers
/// can run these sandboxes more than once.
/// - It implements `DevolvableSandbox`, but not `EvolvableSandbox` so it
/// can be evolved but not devolved
pub trait ReusableSandbox: Sandbox {
    /// Borrow `self` and run this sandbox.
    fn run(&self) -> Result<()>;

    /// Checks if the `Sandbox` needs state resetting.
    fn needs_state_reset(&self) -> bool;

    /// Sets the `Sandbox`'s `needs_state_reset` property to provided value.
    fn set_needs_state_reset(&mut self, val: bool);
}

/// A fully-initialized sandbox that can run guest code or be devolved, but not
/// both. Further, once either operation has occurred, the `OneShotSandbox`
/// cannot be used again.
pub trait OneShotSandbox: Sandbox {
    /// Consume `self` and run the sandbox.
    ///
    /// After this call, you can no longer use this `OneShotSandbox`
    fn run(self) -> Result<()>;
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
