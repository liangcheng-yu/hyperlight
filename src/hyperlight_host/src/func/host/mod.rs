/// Definitions and functionality for supported parameter types in
/// host functions
pub mod param_type;
/// Definitions and functionality for supported return types from
/// guest functions
pub mod ret_type;
/// Functionality to identify and manipulate supported parameter and return
/// values in host functions
pub mod vals;

use anyhow::{bail, Result};
use std::sync::{Arc, Mutex};

use crate::{
    guest_interface_glue::{
        SupportedParameterAndReturnValues, SupportedParameterType, SupportedReturnType,
    },
    sandbox::UnintializedSandbox,
};

pub(crate) type HyperlightFunction<'a> = Arc<
    Mutex<
        Box<
            dyn FnMut(
                    Vec<SupportedParameterAndReturnValues>,
                ) -> anyhow::Result<SupportedParameterAndReturnValues>
                + 'a
                + Send,
        >,
    >,
>;

/// A Hyperlight function that takes no arguments and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function0<'a, R: SupportedReturnType<R>> {
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
}

impl<'a, T, R> Function0<'a, R> for Arc<Mutex<T>>
where
    T: FnMut() -> anyhow::Result<R> + 'a + Send,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |_: Vec<SupportedParameterAndReturnValues>| {
            let result = cloned.lock().unwrap()()?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
    }
}

/// A Hyperlight function that takes 1 argument P1 (which must implement `SupportedParameterType`), and returns an `Anyhow::Result` of type `R` (which must implement `SupportedReturnType`).
pub(crate) trait Function1<
    'a,
    P1: SupportedParameterType<P1> + Clone + 'a,
    R: SupportedReturnType<R>,
>
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
}

impl<'a, T, P1, R> Function1<'a, P1, R> for Arc<Mutex<T>>
where
    T: FnMut(P1) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = Arc::clone(self);
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = P1::get_inner(args[0].clone())?;
            let result = cloned.lock().unwrap()(p1)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
}

impl<'a, T, P1, P2, R> Function2<'a, P1, P2, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let result = cloned.lock().unwrap()(p1, p2)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
}

impl<'a, T, P1, P2, P3, R> Function3<'a, P1, P2, P3, R> for Arc<Mutex<T>>
where
    T: FnMut(P1, P2, P3) -> anyhow::Result<R> + 'a + Send,
    P1: SupportedParameterType<P1> + Clone + 'a,
    P2: SupportedParameterType<P2> + Clone + 'a,
    P3: SupportedParameterType<P3> + Clone + 'a,
    R: SupportedReturnType<R>,
{
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let result = cloned.lock().unwrap()(p1, p2, p3)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let result = cloned.lock().unwrap()(p1, p2, p3, p4)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let result = cloned.lock().unwrap()(p1, p2, p3, p4, p5)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let p6 = P6::get_inner(args[5].clone())?;
            let result = cloned.lock().unwrap()(p1, p2, p3, p4, p5, p6)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let p6 = P6::get_inner(args[5].clone())?;
            let p7 = P7::get_inner(args[6].clone())?;
            let result = cloned.lock().unwrap()(p1, p2, p3, p4, p5, p6, p7)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let p6 = P6::get_inner(args[5].clone())?;
            let p7 = P7::get_inner(args[6].clone())?;
            let p8 = P8::get_inner(args[7].clone())?;
            let result = cloned.lock().unwrap()(p1, p2, p3, p4, p5, p6, p7, p8)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
            let p1 = P1::get_inner(args[0].clone())?;
            let p2 = P2::get_inner(args[1].clone())?;
            let p3 = P3::get_inner(args[2].clone())?;
            let p4 = P4::get_inner(args[3].clone())?;
            let p5 = P5::get_inner(args[4].clone())?;
            let p6 = P6::get_inner(args[5].clone())?;
            let p7 = P7::get_inner(args[6].clone())?;
            let p8 = P8::get_inner(args[7].clone())?;
            let p9 = P9::get_inner(args[8].clone())?;
            let result = cloned.lock().unwrap()(p1, p2, p3, p4, p5, p6, p7, p8, p9)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str);
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
    fn register(&self, sandbox: &mut UnintializedSandbox<'a>, name: &str) {
        let cloned = self.clone();
        let func = Box::new(move |args: Vec<SupportedParameterAndReturnValues>| {
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
            let result = cloned.lock().unwrap()(p1, p2, p3, p4, p5, p6, p7, p8, p9, p10)?;
            Ok(result.get_hyperlight_value())
        });
        sandbox.register_host_function(name, Arc::new(Mutex::new(func)));
    }
}

/// All the types that can be used as parameters or return types for a host
/// function.
pub enum SupportedParameterAndReturnTypes {
    /// i32
    Int,
    /// i64
    Long,
    /// u64
    ULong,
    /// bool
    Bool,
    /// StringF
    String,
    /// Vec<u8>
    ByteArray,
    /// *mut c_void (raw pointer to an unsized type)
    IntPtr,
    /// u32
    UInt,
    /// Void (return types only)
    Void,
}

/// Validates that the given type is supported by the host interface.
pub fn validate_type_supported(some_type: &str) -> Result<()> {
    // try to convert from &str to SupportedParameterAndReturnTypes
    match from_csharp_typename(some_type) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Converts from a C# type name to a SupportedParameterAndReturnTypes.
fn from_csharp_typename(value: &str) -> Result<SupportedParameterAndReturnTypes> {
    match value {
        "System.Int32" => Ok(SupportedParameterAndReturnTypes::Int),
        "System.Int64" => Ok(SupportedParameterAndReturnTypes::Long),
        "System.UInt64" => Ok(SupportedParameterAndReturnTypes::ULong),
        "System.Boolean" => Ok(SupportedParameterAndReturnTypes::Bool),
        "System.String" => Ok(SupportedParameterAndReturnTypes::String),
        "System.Byte[]" => Ok(SupportedParameterAndReturnTypes::ByteArray),
        "System.IntPtr" => Ok(SupportedParameterAndReturnTypes::IntPtr),
        "System.UInt32" => Ok(SupportedParameterAndReturnTypes::UInt),
        other => bail!("Unsupported type: {:?}", other),
    }
}
