use crate::func::{
    function_types::{ParameterValue, ReturnValue},
    host::HyperlightFunction,
};
use crate::sandbox_state::sandbox::Sandbox;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// A `HashMap` to map function names to `HyperlightFunction`s.
pub(crate) type HostFunctionsMap<'a> = HashMap<String, HyperlightFunction<'a>>;

pub(crate) trait HostFuncs<'a>: Sandbox {
    fn get_host_funcs(&self) -> &HostFunctionsMap<'a>;
}

/// Call the host-print function called `"writer_func"`
pub(crate) trait CallHostPrint<'a>: HostFuncs<'a> {
    /// Assuming a host function called `"writer_func"` exists, and takes a
    /// single string parameter, call it with the given `msg` parameter.
    ///
    /// Return `Ok` if the function was found and was of the right signature,
    /// and `Err` otherwise.
    fn host_print(&mut self, msg: String) -> Result<()> {
        call_host_func_impl(
            self.get_host_funcs(),
            "writer_func",
            vec![ParameterValue::String(msg)],
        )?;

        Ok(())
    }
}

/// Generalized functionality to call an arbitrary host function on a `Sandbox`
pub(crate) trait CallHostFunction<'a>: HostFuncs<'a> {
    /// From the set of registered host functions, attempt to get the one
    /// named `name`. If it exists, call it with the given arguments list
    /// `args` and return its result.
    ///
    /// Return `Err` if no such function exists,
    /// its parameter list doesn't match `args`, or there was another error
    /// getting, configuring or calling the function.
    fn call_host_function(&mut self, name: &str, args: Vec<ParameterValue>) -> Result<ReturnValue> {
        call_host_func_impl(self.get_host_funcs(), name, args)
    }
}

fn call_host_func_impl(
    host_funcs: &HostFunctionsMap<'_>,
    name: &str,
    args: Vec<ParameterValue>,
) -> Result<ReturnValue> {
    let func = host_funcs
        .get(name)
        .ok_or_else(|| anyhow!("Host function {} not found", name))?;

    let mut locked_func = func.lock().map_err(|e| anyhow!("error locking: {:?}", e))?;
    locked_func(args)
}
