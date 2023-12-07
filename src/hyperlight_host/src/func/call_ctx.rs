use super::guest_dispatch::dispatch_call_from_host;
#[cfg(feature = "function_call_metrics")]
use crate::histogram_vec_time_micros;
#[cfg(feature = "function_call_metrics")]
use crate::sandbox::metrics::SandboxMetric::GuestFunctionCallDurationMicroseconds;
use crate::{MultiUseSandbox, Result, SingleUseSandbox};
use cfg_if::cfg_if;
use hyperlight_flatbuffers::flatbuffer_wrappers::function_types::{
    ParameterValue, ReturnType, ReturnValue,
};
use std::marker::PhantomData;
use tracing::{instrument, Span};
/// A context for calling guest functions. Can only be created from an
/// existing `MultiUseSandbox`. Once created, guest functions may be made
/// through this and only this context until it is converted back to the
/// `MultiUseSandbox` from which it originated. Upon this conversion,
/// the memory associated with this context will be restored to the same
/// state as before this context was created.
///
/// If dropped, all resources associated with the context and the
/// `MultiUseSandbox` from which it came will be freed.
#[derive(Debug)]
pub struct MultiUseGuestCallContext<'a> {
    sbox: MultiUseSandbox<'a>,
    /// Adding this "marker type" to ensure `MultiUseGuestCallContext` is not
    /// `Send` and thus, instances thereof cannot be sent to a different
    /// thread. This feature is important because it allows us to enlist the
    /// compiler to ensure only one call can ever be in-flight against a
    /// given sandbox at once.
    ///
    /// See https://github.com/rust-lang/rust/issues/68318#issuecomment-1066221968
    /// for more detail
    make_unsend: PhantomData<*mut ()>,
}

impl<'a> MultiUseGuestCallContext<'a> {
    /// Move a `MultiUseSandbox` into a new `CallContext` instance, and
    /// return it
    ///     
    #[instrument(skip_all, parent = Span::current())]
    pub(crate) fn start(sbox: MultiUseSandbox<'a>) -> Self {
        Self {
            sbox,
            make_unsend: PhantomData,
        }
    }

    /// Call the guest function called `func_name` with the given arguments
    /// `args`, and expect the return value have the same type as
    /// `func_ret_type`.
    ///
    /// Every call to this function will be made with the same "context"
    /// resulting from the previous call to this function. This fact
    /// implies the same underlying lock will be held, the state resulting
    /// from the previous call will be present upon this call, and more.
    ///
    /// If you want a "fresh" state, call `finish()` on this `CallContext`
    /// and get a new one from the resulting `MultiUseSandbox`
    #[instrument(err(Debug),skip(self, args),parent = Span::current())]
    pub fn call(
        &mut self,
        func_name: &str,
        func_ret_type: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<ReturnValue> {
        // we are guaranteed to be holding a lock now, since `self` can't
        // exist without doing so. Since GuestCallContext is effectively
        // !Send (and !Sync), we also don't need to worry about
        // synchronization
        cfg_if! {
            if #[cfg(feature = "function_call_metrics")] {
                histogram_vec_time_micros!(
                    &GuestFunctionCallDurationMicroseconds,
                    &[func_name],
                    dispatch_call_from_host(&mut self.sbox, func_name, func_ret_type, args)
                )
            }
            else {
                dispatch_call_from_host(&mut self.sbox, func_name, func_ret_type, args)
            }
        }
    }

    /// Close out the context and get back the internally-stored
    /// `MultiUseSandbox`. Future contexts opened by the returned sandbox
    /// will have a fresh state.
    #[instrument(err(Debug), skip(self), parent = Span::current())]
    pub fn finish(mut self) -> Result<MultiUseSandbox<'a>> {
        self.sbox.reset_state()?;
        Ok(self.sbox)
    }
}

/// A context for calling guest functions. Can only be created from an existing
/// `SingleUseSandbox`, and once created, guest functions against that sandbox
/// can be made from this and only this context until it is dropped.
#[derive(Debug)]
pub struct SingleUseGuestCallContext<'a> {
    sbox: SingleUseSandbox<'a>,
    /// Adding this "marker type" to ensure `SingleUseGuestCallContext` is not `Send`
    /// and thus, instances thereof cannot be sent to a different thread. This
    /// feature is important because it allows us to enlist the compiler to
    /// ensure only one call can ever be in-flight against a given sandbox
    /// at once.
    ///
    /// See https://github.com/rust-lang/rust/issues/68318#issuecomment-1066221968
    /// for more detail on marker types.
    make_unsend: PhantomData<*mut ()>,
}

impl<'a> SingleUseGuestCallContext<'a> {
    /// Move a `SingleUseSandbox` into a new `CallContext` instance, and
    /// return it
    #[instrument(skip_all, parent = Span::current())]
    pub(crate) fn start(sbox: SingleUseSandbox<'a>) -> Self {
        Self {
            sbox,
            make_unsend: PhantomData,
        }
    }

    /// Call the guest function called `func_name` with the given arguments
    /// `args`, and expect the return value have the same type as
    /// `func_ret_type`.
    ///
    /// Every call to this function will be made with the same "context"
    /// resulting from the previous call to this function. This fact
    /// implies the same underlying lock will be held, the state resulting
    /// from the previous call will be present upon this call, and more.
    ///
    /// If you want a "fresh" state, call `finish()` on this `CallContext`
    /// and get a new one from the resulting `MultiUseSandbox`
    #[instrument(err(Debug),skip(self, args),parent = Span::current())]
    pub fn call(
        &mut self,
        func_name: &str,
        func_ret_type: ReturnType,
        args: Option<Vec<ParameterValue>>,
    ) -> Result<ReturnValue> {
        // We are guaranteed to be holding a lock now, since `self` can't
        // exist without doing so. since GuestCallContext is effectively
        // !Send (and !Sync), we also don't need to worry about
        // synchronization

        cfg_if! {
            if #[cfg(feature = "function_call_metrics")] {
                histogram_vec_time_micros!(
                    &GuestFunctionCallDurationMicroseconds,
                    &[func_name],
                    dispatch_call_from_host(&mut self.sbox, func_name, func_ret_type, args)
                )
            }
            else {
                dispatch_call_from_host(&mut self.sbox, func_name, func_ret_type, args)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{sandbox_state::sandbox::EvolvableSandbox, MultiUseSandbox};
    use crate::{sandbox_state::transition::Noop, GuestBinary, HyperlightError};
    use crate::{Result, SingleUseSandbox, UninitializedSandbox};
    use hyperlight_flatbuffers::flatbuffer_wrappers::function_types::{
        ParameterValue, ReturnType, ReturnValue,
    };
    use hyperlight_testing::simple_guest_string;
    use std::sync::mpsc::sync_channel;
    use std::thread::{self, JoinHandle};

    fn new_uninit<'a>() -> Result<UninitializedSandbox<'a>> {
        let path = simple_guest_string().map_err(|e| {
            HyperlightError::Error(format!("failed to get simple guest path ({e:?})"))
        })?;
        UninitializedSandbox::new(GuestBinary::FilePath(path), None, None, None)
    }

    /// Test to create a `SingleUseSandbox`, then call several guest
    /// functions sequentially.
    #[test]
    fn test_single_call() {
        let calls = vec![
            (
                "StackAllocate",
                ReturnType::Int,
                Some(vec![ParameterValue::Int(1)]),
                ReturnValue::Int(1),
            ),
            (
                "CallMalloc",
                ReturnType::Int,
                Some(vec![ParameterValue::Int(200)]),
                ReturnValue::Int(200),
            ),
        ];
        let sbox1: SingleUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
        let mut ctx1 = sbox1.new_call_context();
        for call in calls.iter() {
            let res = ctx1.call(call.0, call.1.clone(), call.2.clone()).unwrap();
            assert_eq!(call.3, res);
        }
    }

    /// Test to create a `MultiUseSandbox`, then call several guest functions
    /// on it across different threads.
    ///
    /// This test works by passing messages between threads using Rust's
    /// [mpsc crate](https://doc.rust-lang.org/std/sync/mpsc). Details of this
    /// interaction are as follows.
    ///
    /// One thread acts as the receiver (AKA: consumer) and owns the
    /// `MultiUseSandbox`. This receiver fields requests from N senders
    /// (AKA: producers) to make batches of calls.
    ///
    /// Upon receipt of a message to execute a batch, a new
    /// `MultiUseGuestCallContext` is created in the receiver thread from the
    /// existing `MultiUseSandbox`, and the batch is executed.
    ///
    /// After the batch is complete, the `MultiUseGuestCallContext` is done
    /// and it is converted back to the underlying `MultiUseSandbox`
    #[test]
    fn test_multi_call_multi_thread() {
        let (snd, recv) = sync_channel::<Vec<TestFuncCall>>(0);

        // create new receiver thread and on it, begin listening for
        // requests to execute batches of calls
        let recv_hdl = thread::spawn(move || {
            let mut sbox: MultiUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
            while let Ok(calls) = recv.recv() {
                let mut ctx = sbox.new_call_context();
                for call in calls {
                    let res = ctx
                        .call(call.func_name.as_str(), call.ret_type, call.params)
                        .unwrap();
                    assert_eq!(call.expected_ret, res);
                }
                sbox = ctx.finish().unwrap();
            }
        });

        // create new sender threads
        let send_handles: Vec<JoinHandle<()>> = (0..10)
            .map(|i| {
                let sender = snd.clone();
                thread::spawn(move || {
                    let calls: Vec<TestFuncCall> = vec![
                        TestFuncCall {
                            func_name: "StackAllocate".to_string(),
                            ret_type: ReturnType::Int,
                            params: Some(vec![ParameterValue::Int(i + 1)]),
                            expected_ret: ReturnValue::Int(i + 1),
                        },
                        TestFuncCall {
                            func_name: "CallMalloc".to_string(),
                            ret_type: ReturnType::Int,
                            params: Some(vec![ParameterValue::Int(i + 2)]),
                            expected_ret: ReturnValue::Int(i + 2),
                        },
                    ];
                    sender.send(calls).unwrap();
                })
            })
            .collect();

        for hdl in send_handles {
            hdl.join().unwrap();
        }
        // after all sender threads are done, drop the sender itself
        // so the receiver thread can exit. then, ensure the receiver
        // thread has exited.
        drop(snd);
        recv_hdl.join().unwrap();
    }

    struct TestFuncCall {
        func_name: String,
        ret_type: ReturnType,
        params: Option<Vec<ParameterValue>>,
        expected_ret: ReturnValue,
    }
}
