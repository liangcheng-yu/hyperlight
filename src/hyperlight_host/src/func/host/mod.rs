/// Definitions and functionality for supported parameter types
pub mod param_type;
/// Definitions and functionality for supported return types
pub mod ret_type;
/// Represents a function call from guest to host.
pub mod function_call;
/// Represents the definition of a function that the host exposes to the guest.
pub mod function_definition;
/// Represents the functions that the host exposes to the guest.
pub mod function_details;

use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::sandbox::UnintializedSandbox;

use self::{
    function_definition::HostFunctionDefinition, param_type::SupportedParameterType,
    ret_type::SupportedReturnType,
};

use super::function_types::{ParameterValue, ReturnValue};

pub(crate) type HyperlightFunction<'a> =
    Arc<Mutex<Box<dyn FnMut(Vec<ParameterValue>) -> anyhow::Result<ReturnValue> + 'a + Send>>>;

/// A Hyperlight function that takes no arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function0<'a, R: SupportedReturnType<R>> {
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, R> Function0<'a, R> for Arc<Mutex<T>>
where
    T: FnMut() -> anyhow::Result<R> + 'a + Send,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = self.clone();
        let func = Box::new(move |_: Vec<ParameterValue>| {
            let result = cloned
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking: {:?}", e))?(
            )?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(
            &HostFunctionDefinition::new(name.to_string(), None, R::get_hyperlight_type()),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 1 argument P1 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function1<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, R> Function1<'a, P1, R> for Arc<Mutex<T>>
where
    T: FnMut(P1) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
        let cloned = Arc::clone(self);
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![P1::get_hyperlight_type()]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 2 arguments P1 and P2 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function2<
    'a,
    P1: SupportedParameterType<P1>,
    P2: SupportedParameterType<P2>,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, R> Function2<'a, P1, P2, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![P1::get_hyperlight_type(), P2::get_hyperlight_type()]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 3 arguments P1, P2 and P3 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function3<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, R> Function3<'a, P1, P2, P3, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![
                    P1::get_hyperlight_type(),
                    P2::get_hyperlight_type(),
                    P3::get_hyperlight_type(),
                ]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 4 arguments P1, P2, P3 and P4 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function4<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, R> Function4<'a, P1, P2, P3, P4, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3, P4) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![
                    P1::get_hyperlight_type(),
                    P2::get_hyperlight_type(),
                    P3::get_hyperlight_type(),
                    P4::get_hyperlight_type(),
                ]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 5 arguments P1, P2, P3, P4 and P5 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function5<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, R> Function5<'a, P1, P2, P3, P4, P5, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3, P4, P5) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    P4: SupportedParameterType<P4> + Clone + 'a,
    P5: SupportedParameterType<P5> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![
                    P1::get_hyperlight_type(),
                    P2::get_hyperlight_type(),
                    P3::get_hyperlight_type(),
                    P4::get_hyperlight_type(),
                    P5::get_hyperlight_type(),
                ]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 6 arguments P1, P2, P3, P4, P5 and P6 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function6<
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, R> Function6<'a, P1, P2, P3, P4, P5, P6, R> for Arc<Mutex<T>>
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![
                    P1::get_hyperlight_type(),
                    P2::get_hyperlight_type(),
                    P3::get_hyperlight_type(),
                    P4::get_hyperlight_type(),
                    P5::get_hyperlight_type(),
                    P6::get_hyperlight_type(),
                ]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 7 arguments P1, P2, P3, P4, P5, P6 and P7 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function7<
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, P7, R> Function7<'a, P1, P2, P3, P4, P5, P6, P7, R>
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![
                    P1::get_hyperlight_type(),
                    P2::get_hyperlight_type(),
                    P3::get_hyperlight_type(),
                    P4::get_hyperlight_type(),
                    P5::get_hyperlight_type(),
                    P6::get_hyperlight_type(),
                    P7::get_hyperlight_type(),
                ]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 8 arguments P1, P2, P3, P4, P5, P6, P7 and P8 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function8<
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, P7, P8, R> Function8<'a, P1, P2, P3, P4, P5, P6, P7, P8, R>
    for Arc<Mutex<T>>
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![
                    P1::get_hyperlight_type(),
                    P2::get_hyperlight_type(),
                    P3::get_hyperlight_type(),
                    P4::get_hyperlight_type(),
                    P5::get_hyperlight_type(),
                    P6::get_hyperlight_type(),
                    P7::get_hyperlight_type(),
                    P8::get_hyperlight_type(),
                ]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 9 arguments P1, P2, P3, P4, P5, P6, P7, P8 and P9 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function9<
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, P7, P8, P9, R>
    Function9<'a, P1, P2, P3, P4, P5, P6, P7, P8, P9, R> for Arc<Mutex<T>>
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![
                    P1::get_hyperlight_type(),
                    P2::get_hyperlight_type(),
                    P3::get_hyperlight_type(),
                    P4::get_hyperlight_type(),
                    P5::get_hyperlight_type(),
                    P6::get_hyperlight_type(),
                    P7::get_hyperlight_type(),
                    P8::get_hyperlight_type(),
                    P9::get_hyperlight_type(),
                ]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}

/// A Hyperlight function that takes 10 arguments P1, P2, P3, P4, P5, P6, P7, P8, P9 and P10 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function10<
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()>;
}

impl<'a, T, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, R>
    Function10<'a, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, R> for Arc<Mutex<T>>
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) -> Result<()> {
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
        sandbox.register_host_function(
            &HostFunctionDefinition::new(
                name.to_string(),
                Some(vec![
                    P1::get_hyperlight_type(),
                    P2::get_hyperlight_type(),
                    P3::get_hyperlight_type(),
                    P4::get_hyperlight_type(),
                    P5::get_hyperlight_type(),
                    P6::get_hyperlight_type(),
                    P7::get_hyperlight_type(),
                    P8::get_hyperlight_type(),
                    P9::get_hyperlight_type(),
                    P10::get_hyperlight_type(),
                ]),
                R::get_hyperlight_type(),
            ),
            Arc::new(Mutex::new(func)),
        )?;

        Ok(())
    }
}
