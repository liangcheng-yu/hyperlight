use crate::{guest_interface_glue::{SupportedParameterAndReturnValues, SupportedReturnType, SupportedParameterType}, sandbox::Sandbox};

pub(crate) type HyperlightFunction = Box<dyn FnMut(Vec<SupportedParameterAndReturnValues>) -> anyhow::Result<SupportedParameterAndReturnValues>>;

/// A Hyperlight function that takes no arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionZero<R: SupportedReturnType> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, R> FunctionZero<R> for T
where
    T: FnMut() -> anyhow::Result<R>,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |_: Vec<SupportedParameterAndReturnValues>| {
            let result = self()?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 1 argument P1 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionOne<P1: SupportedParameterType + Clone + 'static, R: SupportedReturnType> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, R> FunctionOne<P1, R> for T
where
    T: FnMut(P1) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 2 arguments P1 and P2 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionTwo<P1: SupportedParameterType, P2: SupportedParameterType, R: SupportedReturnType> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, P2, R> FunctionTwo<P1, P2, R> for T
where
    T: FnMut(P1, P2) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p2 = args[1].get_inner()?.downcast_ref::<P2>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1, p2)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 3 arguments P1, P2 and P3 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionThree<
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, P2, P3, R> FunctionThree<P1, P2, P3, R> for T
where
    T: FnMut(P1, P2, P3) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p2 = args[1].get_inner()?.downcast_ref::<P2>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p3 = args[2].get_inner()?.downcast_ref::<P3>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1, p2, p3)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 4 arguments P1, P2, P3 and P4 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionFour<
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, P2, P3, P4, R> FunctionFour<P1, P2, P3, P4, R> for T
where
    T: FnMut(P1, P2, P3, P4) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p2 = args[1].get_inner()?.downcast_ref::<P2>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p3 = args[2].get_inner()?.downcast_ref::<P3>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p4 = args[3].get_inner()?.downcast_ref::<P4>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1, p2, p3, p4)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 5 arguments P1, P2, P3, P4 and P5 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionFive<
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, P2, P3, P4, P5, R> FunctionFive<P1, P2, P3, P4, P5, R> for T
where
    T: FnMut(P1, P2, P3, P4, P5) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p2 = args[1].get_inner()?.downcast_ref::<P2>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p3 = args[2].get_inner()?.downcast_ref::<P3>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p4 = args[3].get_inner()?.downcast_ref::<P4>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p5 = args[4].get_inner()?.downcast_ref::<P5>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1, p2, p3, p4, p5)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 6 arguments P1, P2, P3, P4, P5 and P6 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionSix<
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, P2, P3, P4, P5, P6, R> FunctionSix<P1, P2, P3, P4, P5, P6, R> for T
where
    T: FnMut(P1, P2, P3, P4, P5, P6) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p2 = args[1].get_inner()?.downcast_ref::<P2>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p3 = args[2].get_inner()?.downcast_ref::<P3>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p4 = args[3].get_inner()?.downcast_ref::<P4>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p5 = args[4].get_inner()?.downcast_ref::<P5>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p6 = args[5].get_inner()?.downcast_ref::<P6>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1, p2, p3, p4, p5, p6)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 7 arguments P1, P2, P3, P4, P5, P6 and P7 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionSeven<
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    P7: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, P2, P3, P4, P5, P6, P7, R> FunctionSeven<P1, P2, P3, P4, P5, P6, P7, R> for T
where
    T: FnMut(P1, P2, P3, P4, P5, P6, P7) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    P7: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p2 = args[1].get_inner()?.downcast_ref::<P2>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p3 = args[2].get_inner()?.downcast_ref::<P3>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p4 = args[3].get_inner()?.downcast_ref::<P4>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p5 = args[4].get_inner()?.downcast_ref::<P5>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p6 = args[5].get_inner()?.downcast_ref::<P6>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p7 = args[6].get_inner()?.downcast_ref::<P7>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1, p2, p3, p4, p5, p6, p7)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 8 arguments P1, P2, P3, P4, P5, P6, P7 and P8 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionEight<
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    P7: SupportedParameterType + Clone + 'static,
    P8: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, P2, P3, P4, P5, P6, P7, P8, R> FunctionEight<P1, P2, P3, P4, P5, P6, P7, P8, R> for T
where
    T: FnMut(P1, P2, P3, P4, P5, P6, P7, P8) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    P7: SupportedParameterType + Clone + 'static,
    P8: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p2 = args[1].get_inner()?.downcast_ref::<P2>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p3 = args[2].get_inner()?.downcast_ref::<P3>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p4 = args[3].get_inner()?.downcast_ref::<P4>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p5 = args[4].get_inner()?.downcast_ref::<P5>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p6 = args[5].get_inner()?.downcast_ref::<P6>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p7 = args[6].get_inner()?.downcast_ref::<P7>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p8 = args[7].get_inner()?.downcast_ref::<P8>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1, p2, p3, p4, p5, p6, p7, p8)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 9 arguments P1, P2, P3, P4, P5, P6, P7, P8 and P9 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionNine<
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    P7: SupportedParameterType + Clone + 'static,
    P8: SupportedParameterType + Clone + 'static,
    P9: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, P2, P3, P4, P5, P6, P7, P8, P9, R> FunctionNine<P1, P2, P3, P4, P5, P6, P7, P8, P9, R>
    for T
where
    T: FnMut(P1, P2, P3, P4, P5, P6, P7, P8, P9) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    P7: SupportedParameterType + Clone + 'static,
    P8: SupportedParameterType + Clone + 'static,
    P9: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p2 = args[1].get_inner()?.downcast_ref::<P2>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p3 = args[2].get_inner()?.downcast_ref::<P3>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p4 = args[3].get_inner()?.downcast_ref::<P4>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p5 = args[4].get_inner()?.downcast_ref::<P5>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p6 = args[5].get_inner()?.downcast_ref::<P6>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p7 = args[6].get_inner()?.downcast_ref::<P7>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p8 = args[7].get_inner()?.downcast_ref::<P8>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p9 = args[8].get_inner()?.downcast_ref::<P9>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1, p2, p3, p4, p5, p6, p7, p8, p9)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}

/// A Hyperlight function that takes 10 arguments P1, P2, P3, P4, P5, P6, P7, P8, P9 and P10 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait FunctionTen<
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    P7: SupportedParameterType + Clone + 'static,
    P8: SupportedParameterType + Clone + 'static,
    P9: SupportedParameterType + Clone + 'static,
    P10: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
> {
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str);
}

impl<T, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, R> FunctionTen<
    P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, R,
> for T
where
    T: FnMut(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10) -> anyhow::Result<R>,
    P1: SupportedParameterType + Clone + 'static,
    P2: SupportedParameterType + Clone + 'static,
    P3: SupportedParameterType + Clone + 'static,
    P4: SupportedParameterType + Clone + 'static,
    P5: SupportedParameterType + Clone + 'static,
    P6: SupportedParameterType + Clone + 'static,
    P7: SupportedParameterType + Clone + 'static,
    P8: SupportedParameterType + Clone + 'static,
    P9: SupportedParameterType + Clone + 'static,
    P10: SupportedParameterType + Clone + 'static,
    R: SupportedReturnType,
{
    fn register(&'static mut self, sandbox: &mut Sandbox, name: &str) {
        let boxed = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = args[0].get_inner()?.downcast_ref::<P1>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p2 = args[1].get_inner()?.downcast_ref::<P2>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p3 = args[2].get_inner()?.downcast_ref::<P3>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p4 = args[3].get_inner()?.downcast_ref::<P4>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p5 = args[4].get_inner()?.downcast_ref::<P5>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p6 = args[5].get_inner()?.downcast_ref::<P6>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p7 = args[6].get_inner()?.downcast_ref::<P7>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p8 = args[7].get_inner()?.downcast_ref::<P8>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p9 = args[8].get_inner()?.downcast_ref::<P9>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let p10 = args[9].get_inner()?.downcast_ref::<P10>().cloned().ok_or(anyhow::anyhow!("Invalid parameter type"))?;
            let result = self(p1, p2, p3, p4, p5, p6, p7, p8, p9, p10)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.host_functions.insert(name.to_string(), boxed);
    }
}