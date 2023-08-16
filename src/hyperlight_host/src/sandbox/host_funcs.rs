use crate::{
    func::{
        host::function_definition::HostFunctionDefinition,
        types::{ParameterValue, ReturnValue},
        HyperlightFunction,
    },
    mem::mgr::SandboxMemoryManager,
};
use anyhow::{anyhow, Result};
use is_terminal::IsTerminal;
use std::io::stdout;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use super::FunctionsMap;

#[derive(Default, Clone)]
pub(crate) struct HostFuncsWrapper<'a>(FunctionsMap<'a>);

impl<'a> From<FunctionsMap<'a>> for HostFuncsWrapper<'a> {
    fn from(funcs_map: FunctionsMap<'a>) -> Self {
        Self(funcs_map)
    }
}

impl<'a> HostFuncsWrapper<'a> {
    fn get_host_funcs(&self) -> &FunctionsMap<'a> {
        &self.0
    }

    fn get_host_funcs_mut(&mut self) -> &mut FunctionsMap<'a> {
        &mut self.0
    }

    pub(super) fn len(&self) -> usize {
        self.0.len()
    }

    /// Register a host function with the sandbox.
    pub(crate) fn register_host_function(
        &mut self,
        mgr: &mut SandboxMemoryManager,
        hfd: &HostFunctionDefinition,
        func: HyperlightFunction<'a>,
    ) -> Result<()> {
        self.get_host_funcs_mut()
            .insert(hfd.function_name.to_string(), func);
        let buffer: Vec<u8> = hfd.try_into()?;
        mgr.write_host_function_definition(&buffer)?;
        Ok(())
    }

    /// Assuming a host function called `"writer_func"` exists, and takes a
    /// single string parameter, call it with the given `msg` parameter.
    ///
    /// Return `Ok` if the function was found and was of the right signature,
    /// and `Err` otherwise.
    pub(crate) fn host_print(&mut self, msg: String) -> Result<()> {
        call_host_func_impl(
            self.get_host_funcs(),
            "writer_func",
            vec![ParameterValue::String(msg)],
        )?;

        Ok(())
    }
    /// From the set of registered host functions, attempt to get the one
    /// named `name`. If it exists, call it with the given arguments list
    /// `args` and return its result.
    ///
    /// Return `Err` if no such function exists,
    /// its parameter list doesn't match `args`, or there was another error
    /// getting, configuring or calling the function.
    pub(super) fn call_host_function(
        &self,
        name: &str,
        args: Vec<ParameterValue>,
    ) -> Result<ReturnValue> {
        call_host_func_impl(self.get_host_funcs(), name, args)
    }
}

fn call_host_func_impl(
    host_funcs: &FunctionsMap<'_>,
    name: &str,
    args: Vec<ParameterValue>,
) -> Result<ReturnValue> {
    let func = host_funcs
        .get(name)
        .ok_or_else(|| anyhow!("Host function {} not found", name))?;

    func.call(args)
}

// The default writer function is to write to stdout with green text.
pub(crate) fn default_writer_func(s: String) -> Result<()> {
    match stdout().is_terminal() {
        false => {
            print!("{}", s);
            Ok(())
        }
        true => {
            let mut stdout = StandardStream::stdout(ColorChoice::Auto);
            let mut color_spec = ColorSpec::new();
            color_spec.set_fg(Some(Color::Green));
            stdout.set_color(&color_spec)?;
            stdout.write_all(s.as_bytes())?;
            stdout.reset()?;
            Ok(())
        }
    }
}
