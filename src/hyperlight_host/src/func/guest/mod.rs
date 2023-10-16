/// Represents an error that occured int the guest.
pub mod error;
/// Represents a function call from host to guest.
pub mod function_call;
/// Represents the definition of a function that the guest exposes to the host.
pub(crate) mod function_definition;
/// Represents the functions that the guest exposes to the host.
pub(crate) mod function_details;
/// Represents guest log data
pub mod log_data;
/// An enumeration and supporting logic to determine the desired
/// level of a log message issued from the guest.
pub(crate) mod log_level;

use crate::Result;
use crate::{sandbox::initialized_single_use::SingleUseSandbox, MultiUseSandbox};
use std::sync::{Arc, Mutex};

/// A simple guest function with no arguments and an `Result<Res>`
/// return type, executable by an `InSbox` type
pub trait GuestFunction<InSbox, Res> {
    /// Call the guest function in `self` and using the `s` parameter, call
    /// the function and return the following:
    ///
    /// - `Ok(res)` - if the call was successful and `res` was returned
    /// - `Err(e)` - if the call failed. `e` is an error with details on
    /// the failure
    fn call(&self, s: InSbox) -> Result<Res>;
}

/// The implementation of `GuestFunction` for a `FnMut` that takes a
/// `SingleUseSandbox` as its parameter, and is executable by a
/// `SingleUseSandbox`.
///
/// Importantly, the `SingleUseSandbox` passed to this guest function is
/// _moved_ into it, so the type system ensures the given sandbox can only
/// ever execute _one_ function call before being destroyed
///
/// Sample usage is as follows:
///
/// ```rust
/// use hyperlight_host::Result;
/// use hyperlight_host::func::guest::GuestFunction;
/// use hyperlight_host::SingleUseSandbox;
/// use std::sync::{Mutex, Arc};
///
/// // first, create our function. at this point, its type will be
/// // `Arc<Mutex<FnOnce(SingleUseSandbox) -> Result<u64>`
/// let my_func = Arc::new(Mutex::new(|su_sbox: SingleUseSandbox| -> Result<u64> {
///     println!("inside my single use guest func!");
///     // here is where we would utilize `su_sbox` to do guest calls
///     Ok(100_u64)
/// }));
/// // next, assume we have a `sbox` variable that holds a
/// // `SingleUseSandbox`, and make the call.
///
/// // since we've imported `SingleUseGuestFunction` above, our `my_func`
/// // variable will pick up the `GuestFunction` impl, and we will have
/// // the `call` method available to execute the function.
/// //
/// // this line is commented this out since we don't actually have an
/// // actual sandbox
/// // let call_res: Result<u64> = my_func.call(sbox);
///
/// // a second call will fail to compile:
/// // let call_res_2: Result<u64> = my_func.call(sbox);
/// ```
impl<'sbox, 'func, FuncT, ResT> GuestFunction<SingleUseSandbox<'sbox>, ResT> for Arc<Mutex<FuncT>>
where
    FuncT: FnMut(SingleUseSandbox<'sbox>) -> Result<ResT> + 'func + Send,
{
    fn call(&self, s: SingleUseSandbox<'sbox>) -> Result<ResT> {
        call_impl(self, s)
    }
}

/// The implementation of `GuestFunction`  for a `FnMut` that takes an
/// `Arc<Mutex<MultiUseSandbox>>` as its parameter, and is executable by
/// a `MultiUseSandbox`.
///
/// Importantly, the `MultiUseSandbox` is passed to this guest function as
/// an `Arc<Mutex<MultiUseSandbox<'sbox>>>`, indicating the
/// `MultiUseSandbox` can be shared across multiple function calls,
/// potentially across threads
///
/// The type for which this `impl` implements `GuestFunction` is complex
/// and for clarity is written in full as follows:
///
/// `Arc<Mutex<FnOnce(Arc<Mutex<MutiUseSandbox>>) -> Result<ResT>>>`
///
/// Sample usage of `MultiUseGuestFunction` is as follows:
///
/// ```rust
/// use hyperlight_host::Result;
/// use hyperlight_host::MultiUseSandbox;
/// use hyperlight_host::func::guest::GuestFunction;
/// use std::sync::{Arc, Mutex};
///
/// // first, create our function. at this point, its type will be
/// // `Arc<Mutex<FnOnce(Arc<Mutex<MultiUseSandbox>>) -> Result<u64>`
/// let my_func = Arc::new(Mutex::new(|su_sbox: Arc<Mutex<MultiUseSandbox>>| -> Result<u64> {
///     println!("inside my multi use guest func!");
///     // here is where we would utilize `su_sbox` to do guest calls
///     Ok(100_u64)
/// }));
///
/// // next, assume we have a `sbox` variable that holds an
/// // `Arc<Mutex<MultiUseSandbox>>`, and make the call.
/// //
/// // since we've imported `MultiUseGuestFunction` above, our `my_func`
/// // variable will pick up the `GuestFunction` impl, and we will have
/// // the `call` method available to execute the function.
/// //
/// // This line is commented out since we don't have an actual sandbox
/// // available here
/// // let call_res: Result<u64> = my_func.call(sbox);
/// ```
pub trait MultiUseGuestFunction<'sbox, ResT>:
    GuestFunction<Arc<Mutex<MultiUseSandbox<'sbox>>>, ResT>
{
}

/// A simple guest function with no arguments and an `Result<R>`
/// return type, executable by a `MultiUseSandbox`
impl<'sbox, 'func, FuncT, ResT> GuestFunction<Arc<Mutex<MultiUseSandbox<'sbox>>>, ResT>
    for Arc<Mutex<FuncT>>
where
    FuncT: FnMut(Arc<Mutex<MultiUseSandbox<'sbox>>>) -> Result<ResT> + 'func + Send,
{
    fn call(&self, s: Arc<Mutex<MultiUseSandbox<'sbox>>>) -> Result<ResT> {
        call_impl(self, s)
    }
}

fn call_impl<'func, SboxT, FuncT, ResT>(func: &Arc<Mutex<FuncT>>, sbox: SboxT) -> Result<ResT>
where
    FuncT: FnMut(SboxT) -> Result<ResT> + 'func + Send,
{
    let mut func = func.lock()?;
    func(sbox)
}
