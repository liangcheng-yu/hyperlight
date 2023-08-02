/// Represents an error that occured int the guest.
pub(crate) mod error;
/// Represents a function call from host to guest.
pub(crate) mod function_call;
/// Represents the definition of a function that the guest exposes to the host.
pub(crate) mod function_definition;
/// Represents the functions that the guest exposes to the host.
pub(crate) mod function_details;
/// Represents guest log data
pub(crate) mod log_data;
/// An enumeration and supporting logic to determine the desired
/// level of a log message issued from the guest.
pub(crate) mod log_level;

use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::{sandbox::guest_funcs::GuestFuncs, UninitializedSandbox};

use super::{
    param_type::SupportedParameterType, ret_type::SupportedReturnType, types::ParameterValue,
};

/// A simple guest function that takes no arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub trait GuestFunction<R> {
    fn call(&self) -> Result<R>;
}

impl<'a, T, R> GuestFunction<R> for Arc<Mutex<T>>
where
    T: FnMut() -> anyhow::Result<R> + 'a + Send,
{
    fn call(&self) -> Result<R> {
        let result = self
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?()?;
        Ok(result)
    }
}

/// A dynamic guest function that takes no arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction0<'a, R: SupportedReturnType<R>> {
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, R> DynamicGuestFunction0<'a, R> for Arc<Mutex<T>>
where
    T: FnMut() -> anyhow::Result<R> + 'a + Send,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |_: Vec<ParameterValue>| {
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes a single argument and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction1<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, R> DynamicGuestFunction1<'a, P1, R> for Arc<Mutex<T>>
where
    T: FnMut(P1) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 1 {
                return Err(anyhow::anyhow!("Expected 1 argument, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let result =
                cloned
                    .lock()
                    .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(p1)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes two arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction2<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, R> DynamicGuestFunction2<'a, P1, P2, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 2 {
                return Err(anyhow::anyhow!("Expected 2 arguments, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
                p1, p2
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes three arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction3<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, R> DynamicGuestFunction3<'a, P1, P2, P3, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 3 {
                return Err(anyhow::anyhow!("Expected 3 arguments, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
                p1, p2, p3
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes four arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction4<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, R> DynamicGuestFunction4<'a, P1, P2, P3, P4, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3, P4) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 4 {
                return Err(anyhow::anyhow!("Expected 4 arguments, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
                p1, p2, p3, p4
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes five arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction5<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, R> DynamicGuestFunction5<'a, P1, P2, P3, P4, P5, R>
    for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3, P4, P5) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 5 {
                return Err(anyhow::anyhow!("Expected 5 arguments, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
                p1, p2, p3, p4, p5
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes six arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction6<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, R> DynamicGuestFunction6<'a, P1, P2, P3, P4, P5, P6, R>
    for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3, P4, P5, P6) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 6 {
                return Err(anyhow::anyhow!("Expected 6 arguments, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let p6 = P6::get_inner(args[5].clone())?;
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
                p1, p2, p3, p4, p5, p6
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes seven arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction7<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    P7: SupportedParameterType<P7> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, P7, R> DynamicGuestFunction7<'a, P1, P2, P3, P4, P5, P6, P7, R>
    for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3, P4, P5, P6, P7) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    P7: SupportedParameterType<P7> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 7 {
                return Err(anyhow::anyhow!("Expected 7 arguments, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let p6 = P6::get_inner(args[5].clone())?;
            let p7 = P7::get_inner(args[6].clone())?;
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
                p1, p2, p3, p4, p5, p6, p7,
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes eight arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction8<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    P7: SupportedParameterType<P7> + Clone + 'a,
    P8: SupportedParameterType<P8> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, P7, P8, R>
    DynamicGuestFunction8<'a, P1, P2, P3, P4, P5, P6, P7, P8, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3, P4, P5, P6, P7, P8) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    P7: SupportedParameterType<P7> + Clone + 'a,
    P8: SupportedParameterType<P8> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 8 {
                return Err(anyhow::anyhow!("Expected 8 arguments, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let p6 = P6::get_inner(args[5].clone())?;
            let p7 = P7::get_inner(args[6].clone())?;
            let p8 = P8::get_inner(args[7].clone())?;
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
                p1, p2, p3, p4, p5, p6, p7, p8,
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes nine arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction9<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    P7: SupportedParameterType<P7> + Clone + 'a,
    P8: SupportedParameterType<P8> + Clone + 'a,
    P9: SupportedParameterType<P9> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, P7, P8, P9, R>
    DynamicGuestFunction9<'a, P1, P2, P3, P4, P5, P6, P7, P8, P9, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3, P4, P5, P6, P7, P8, P9) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    P7: SupportedParameterType<P7> + Clone + 'a,
    P8: SupportedParameterType<P8> + Clone + 'a,
    P9: SupportedParameterType<P9> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 9 {
                return Err(anyhow::anyhow!("Expected 9 arguments, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let p6 = P6::get_inner(args[5].clone())?;
            let p7 = P7::get_inner(args[6].clone())?;
            let p8 = P8::get_inner(args[7].clone())?;
            let p9 = P9::get_inner(args[8].clone())?;
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
                p1, p2, p3, p4, p5, p6, p7, p8, p9,
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}

/// A dynamic guest function that takes ten arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait DynamicGuestFunction10<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    P7: SupportedParameterType<P7> + Clone + 'a,
    P8: SupportedParameterType<P8> + Clone + 'a,
    P9: SupportedParameterType<P9> + Clone + 'a,
    P10: SupportedParameterType<P10> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, R>
    DynamicGuestFunction10<'a, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    P6: SupportedParameterType<P6> + Clone + 'a,
    P7: SupportedParameterType<P7> + Clone + 'a,
    P8: SupportedParameterType<P8> + Clone + 'a,
    P9: SupportedParameterType<P9> + Clone + 'a,
    P10: SupportedParameterType<P10> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UninitializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<ParameterValue>| {
            if args.len() != 10 {
                return Err(anyhow::anyhow!("Expected 10 arguments, got {}", args.len()));
            }
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let p6 = P6::get_inner(args[5].clone())?;
            let p7 = P7::get_inner(args[6].clone())?;
            let p8 = P8::get_inner(args[7].clone())?;
            let p9 = P9::get_inner(args[8].clone())?;
            let p10 = P10::get_inner(args[9].clone())?;
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
                p1, p2, p3, p4, p5, p6, p7, p8, p9, p10,
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.add_dynamic_method(name, Arc::new(Mutex::new(func)))?;

        Ok(())
    }
}
