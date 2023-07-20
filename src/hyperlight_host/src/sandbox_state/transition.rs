use std::marker::PhantomData;

use super::sandbox::Sandbox;
use anyhow::Result;

/// Metadata about an evolution or devolution. Any `Sandbox` implementation
/// that also implements `EvolvableSandbox` or `DevolvableSandbox`
/// can decide the following things in a type-safe way:
///
/// 1. That transition is possible
/// 2. That transition requires a specific kind of metadata
///
/// For example, if you have the following structs:
///
/// ```rust
/// struct MySandbox1 {}
/// struct MySandbox2 {}
///
/// impl Sandbox for MySandbox1 {...}
/// impl Sandbox for MySandbox2 {...}
/// ```
///
/// ...then you can define a metadata-free evolve transition between
/// `MySandbox1` and `MySandbox2`, and a devolve transition that requires
/// a callback between `MySandbox2` and `MySandbox` as follows:
///
/// ```rust
/// impl EvolvableSandbox<
///     MySandbox1,
///     MySandbox2,
///     Noop<MySandbox1, MySandbox2>
/// > for MySandbox1 {
///     fn evolve(
///         self,
///         _: Noop<MySandbox1, MySandbox2>
///     ) -> Result<MySandbox2> {
///         Ok(MySandbox2{})
///     }
/// }
///
/// impl<F> DevolvableSandbox<
///     MySandbox2,
///     MySandbox1,
///     MutatingCallback<MySandbox2, F>
/// > for MySandbox2
/// where F: FnOnce(&mut Cur) -> Result<()>,
/// {
///     fn devolve(
///         self,
///         cb: MutatingCallback<MySandbox2, F>
///     ) -> Result<MySandbox1> {
///         cb.call(self)?;
///         Ok(MySandbox1{})
///     }
/// }
///
/// ```
///
/// Most transitions will likely involve `Noop`, but some may involve
/// `MutatingCallback` or even implement their own.
pub trait TransitionMetadata<Cur: Sandbox, Next: Sandbox> {}

/// Transition metadata that contains and does nothing. `Noop` is a
/// placeholder when you want to implement an `EvolvableSandbox`
/// or `DevolvableSandbox` that needs no additional metadata to succeed.
///
/// Construct one of these by using the `default()` method.
pub struct Noop<Cur: Sandbox, Next: Sandbox> {
    cur_ph: PhantomData<Cur>,
    next_ph: PhantomData<Next>,
}

impl<Cur: Sandbox, Next: Sandbox> Default for Noop<Cur, Next> {
    fn default() -> Self {
        Self {
            cur_ph: PhantomData,
            next_ph: PhantomData,
        }
    }
}

impl<Cur: Sandbox, Next: Sandbox> TransitionMetadata<Cur, Next> for Noop<Cur, Next> {}

/// A `TransitionMetadata` that calls a callback. Most `EvolvableSandbox`
/// or `DevolvableSandbox` implementations will want to pass their `self`
/// parameter to the `MutatingCallback::call` method prior to taking any
/// other action, but the order in which this happens is completely up
/// to the implementor.
///
/// The `call` method returns an `anyhow::Result<()>`, which is intended
/// to signal whether the callback was successful. Although the implementor of
/// `EvolvableSandbox`/`DevolvableSandbox` will ultimately choose what to
/// do in case of failure, it's recommended they fail the `evolve`/`devolve`
/// operation immediately, rather than proceeding with a potentially
/// invalid operation.
///
/// Construct one of these by passing your callback to
/// `MutatingCallback::from`, as in the following code (assuming `MySandbox`
/// is a `Sandbox` implementation):
///
/// ```rust
/// let my_cb_fn: FnOnce(&mut MySandbox) -> anyhow::Result<()> = |_sbox| {
///     println!("hello world!");
/// };
/// let mutating_cb = MutatingCallback::from(my_cb_fn);
/// ```
pub struct MutatingCallback<'func, Cur: Sandbox, F>
where
    F: FnOnce(&mut Cur) -> Result<()> + 'func,
{
    cur_ph: PhantomData<Cur>,
    fn_life_ph: PhantomData<&'func ()>,
    cb: F,
}

impl<'a, Cur: Sandbox, Next: Sandbox, F> TransitionMetadata<Cur, Next>
    for MutatingCallback<'a, Cur, F>
where
    F: FnOnce(&mut Cur) -> Result<()>,
{
}

impl<'a, Cur: Sandbox, F> MutatingCallback<'a, Cur, F>
where
    F: FnOnce(&mut Cur) -> Result<()>,
{
    pub(crate) fn call(self, cur: &mut Cur) -> Result<()> {
        (self.cb)(cur)
    }
}

impl<'a, Cur: Sandbox, F> From<F> for MutatingCallback<'a, Cur, F>
where
    F: FnOnce(&mut Cur) -> Result<()> + 'a,
{
    fn from(val: F) -> Self {
        MutatingCallback {
            cur_ph: PhantomData,
            fn_life_ph: PhantomData,
            cb: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MutatingCallback, Noop};
    use crate::sandbox_state::sandbox::{DevolvableSandbox, EvolvableSandbox, Sandbox};
    use anyhow::Result;

    #[derive(Debug, Eq, PartialEq, Clone)]
    struct MySandbox1 {}
    #[derive(Debug, Eq, PartialEq, Clone)]
    struct MySandbox2 {}
    impl Sandbox for MySandbox1 {}
    impl Sandbox for MySandbox2 {}

    impl EvolvableSandbox<MySandbox1, MySandbox2, Noop<MySandbox1, MySandbox2>> for MySandbox1 {
        fn evolve(self, _: Noop<MySandbox1, MySandbox2>) -> Result<MySandbox2> {
            Ok(MySandbox2 {})
        }
    }

    impl<'cb, F> DevolvableSandbox<MySandbox2, MySandbox1, MutatingCallback<'cb, MySandbox2, F>>
        for MySandbox2
    where
        F: FnOnce(&mut MySandbox2) -> Result<()> + 'cb,
    {
        fn devolve(mut self, cb: MutatingCallback<MySandbox2, F>) -> Result<MySandbox1> {
            cb.call(&mut self)?;
            Ok(MySandbox1 {})
        }
    }

    #[test]
    fn test_evolve_devolve() {
        let sbox_1_1 = MySandbox1 {};
        let sbox_2_1 = sbox_1_1.clone().evolve(Noop::default()).unwrap();
        let sbox_1_2 = sbox_2_1
            .clone()
            .devolve(MutatingCallback::from(Box::new(
                |_: &mut MySandbox2| Ok(()),
            )))
            .unwrap();
        let sbox_2_2 = sbox_1_2.clone().evolve(Noop::default()).unwrap();
        assert_eq!(sbox_1_1, sbox_1_2);
        assert_eq!(sbox_2_1, sbox_2_2);
    }
}
