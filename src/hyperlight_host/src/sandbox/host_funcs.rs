use std::io::{stdout, Write};

use hyperlight_common::flatbuffer_wrappers::function_types::{ParameterValue, ReturnValue};
use hyperlight_common::flatbuffer_wrappers::host_function_definition::HostFunctionDefinition;
use hyperlight_common::flatbuffer_wrappers::host_function_details::HostFunctionDetails;
use is_terminal::IsTerminal;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tracing::{instrument, Span};

use super::FunctionsMap;
use crate::func::HyperlightFunction;
use crate::mem::mgr::SandboxMemoryManager;
use crate::HyperlightError::HostFunctionNotFound;
use crate::{new_error, Result};

#[derive(Default, Clone)]
/// A Wrapper around details of functions exposed by the Host
pub struct HostFuncsWrapper {
    functions_map: FunctionsMap,
    function_details: HostFunctionDetails,
}

impl HostFuncsWrapper {
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_host_funcs(&self) -> &FunctionsMap {
        &self.functions_map
    }
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_host_funcs_mut(&mut self) -> &mut FunctionsMap {
        &mut self.functions_map
    }
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_host_func_details(&self) -> &HostFunctionDetails {
        &self.function_details
    }
    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn get_host_func_details_mut(&mut self) -> &mut HostFunctionDetails {
        &mut self.function_details
    }

    /// Register a host function with the sandbox.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub(crate) fn register_host_function(
        &mut self,
        mgr: &mut SandboxMemoryManager,
        hfd: &HostFunctionDefinition,
        func: HyperlightFunction,
    ) -> Result<()> {
        self.get_host_funcs_mut()
            .insert(hfd.function_name.to_string(), func);
        self.get_host_func_details_mut()
            .insert_host_function(hfd.clone());
        // Functions need to be sorted so that they are serialised in sorted order
        // this is required in order for flatbuffers C implementation used in the Gues Library
        // to be able to search the functions by name.
        self.get_host_func_details_mut()
            .sort_host_functions_by_name();
        let buffer: Vec<u8> = self.get_host_func_details().try_into().map_err(|e| {
            new_error!(
                "Error serializing host function details to flatbuffer: {}",
                e
            )
        })?;
        mgr.write_buffer_host_function_details(&buffer)?;

        Ok(())
    }

    /// Assuming a host function called `"HostPrint"` exists, and takes a
    /// single string parameter, call it with the given `msg` parameter.
    ///
    /// Return `Ok` if the function was found and was of the right signature,
    /// and `Err` otherwise.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub(super) fn host_print(&mut self, msg: String) -> Result<i32> {
        let res = call_host_func_impl(
            self.get_host_funcs(),
            "HostPrint",
            vec![ParameterValue::String(msg)],
        )?;
        res.try_into()
            .map_err(|_| HostFunctionNotFound("HostPrint".to_string()))
    }
    /// From the set of registered host functions, attempt to get the one
    /// named `name`. If it exists, call it with the given arguments list
    /// `args` and return its result.
    ///
    /// Return `Err` if no such function exists,
    /// its parameter list doesn't match `args`, or there was another error
    /// getting, configuring or calling the function.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub(super) fn call_host_function(
        &self,
        name: &str,
        args: Vec<ParameterValue>,
    ) -> Result<ReturnValue> {
        call_host_func_impl(self.get_host_funcs(), name, args)
    }
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
fn call_host_func_impl(
    host_funcs: &FunctionsMap,
    name: &str,
    args: Vec<ParameterValue>,
) -> Result<ReturnValue> {
    let func = host_funcs
        .get(name)
        .ok_or_else(|| HostFunctionNotFound(name.to_string()))?;

    #[cfg(feature = "function_call_metrics")]
    {
        let start = std::time::Instant::now();
        let result = func.call(args);
        crate::histogram_vec_observe!(
            &crate::sandbox::metrics::SandboxMetric::HostFunctionCallsDurationMicroseconds,
            &[name],
            start.elapsed().as_micros() as f64
        );
        result
    }

    #[cfg(not(feature = "function_call_metrics"))]
    func.call(args)
}

/// The default writer function is to write to stdout with green text.
#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
pub(super) fn default_writer_func(s: String) -> Result<i32> {
    match stdout().is_terminal() {
        false => {
            print!("{}", s);
            Ok(s.len() as i32)
        }
        true => {
            let mut stdout = StandardStream::stdout(ColorChoice::Auto);
            let mut color_spec = ColorSpec::new();
            color_spec.set_fg(Some(Color::Green));
            stdout.set_color(&color_spec)?;
            stdout.write_all(s.as_bytes())?;
            stdout.reset()?;
            Ok(s.len() as i32)
        }
    }
}
