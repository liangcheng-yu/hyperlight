use super::sandbox_run_options::SandboxRunOptions;
use crate::flatbuffers::hyperlight::generated::ErrorCode;
use crate::guest::guest_log_data::GuestLogData;
use crate::guest::log_level::LogLevel;
use crate::guest_interface_glue::{
    HostMethodInfo, SupportedParameterAndReturnTypes, SupportedParameterAndReturnValues,
};
use crate::hypervisor::Hypervisor;
use crate::mem::mgr::STACK_COOKIE_LEN;
use crate::mem::ptr::RawPtr;
use crate::mem::{
    config::SandboxMemoryConfiguration, mgr::SandboxMemoryManager, pe::pe_info::PEInfo,
};
#[cfg(target_os = "linux")]
use crate::{
    hypervisor::hyperv_linux::{self, HypervLinuxDriver, REQUIRE_STABLE_API},
    hypervisor::hypervisor_mem::HypervisorAddrs,
    hypervisor::kvm,
    hypervisor::kvm::KVMDriver,
    mem::layout::SandboxMemoryLayout,
    mem::ptr::GuestPtr,
    mem::ptr_offset::Offset,
};
use anyhow::{anyhow, bail, Result};
use log::{debug, error, info, trace, warn};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::io::stdout;
use std::io::Write;
use std::ops::Add;
use std::option::Option;
use std::path::Path;
use std::rc::Rc;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

// In case its not obvious why there are separate is_supported_platform and is_hypervisor_present functions its because
// Hyperlight is designed to be able to run on a host that doesn't have a hypervisor.
// In that case, the sandbox will be in process, we plan on making this a dev only feature and fixing up Linux support
// so we should review the need for this function at that time.

/// Determine if this is a supported platform for Hyperlight
///
/// Returns a boolean indicating whether this is a supported platform.
///
pub(crate) fn is_supported_platform() -> bool {
    #[cfg(not(target_os = "linux"))]
    #[cfg(not(target_os = "windows"))]
    return false;

    true
}

enum OutBAction {
    Log,
    CallFunction,
    Abort,
}

impl From<u16> for OutBAction {
    fn from(val: u16) -> Self {
        match val {
            99 => OutBAction::Log,
            101 => OutBAction::CallFunction,
            102 => OutBAction::Abort,
            _ => OutBAction::Log,
        }
    }
}

/// Determine whether a suitable hypervisor is available to run
/// this sandbox.
///
//  Returns a boolean indicating whether a suitable hypervisor is present.

// TODO - implement this
pub(crate) fn is_hypervisor_present() -> bool {
    #[cfg(target_os = "linux")]
    return true;
    #[cfg(target_os = "windows")]
    return true;
    #[cfg(not(target_os = "linux"))]
    #[cfg(not(target_os = "windows"))]
    false
}

// PrintOutputFunctionPointer is a pointer to a function in the host that can be called from the Sandbox
// it is defined as Rc<RefCell<dyn FnMut(String) -> Result<() +'a>>>,

// Rc as the function pointer is shared between the sandbox and the host
// RefCell as the function pointer needs to be able to be shared (potentially mutably) between the sandbox and the host
// dyn FnMut as the function pointer can be a closure that can mutate captured variables
// 'a so the function pointer does not have a static lifetime by default

// The actual function it points to can be one of the following:
//
// fn. A static function in the host.
// Fn. A closure in the host that can reference captured context.
// FnMut. A closure in the host that can mutate captured context.

// PrintOutputFunctionPointer is a pointer to a print_output function in the host that can be called from the Sandbox in place of the deault behaviour of writing to stdout.
// pub type PrintOutputFunctionPointer<'a> = Rc<RefCell<dyn FnMut(String) -> Result<()> + 'a>>;

// However this should be generic so that it can be used for any host function not a special case for print_output
// One way of doing this is as follows:

/// This is a marker trait that is used to indicate that a type is a valid Hyperlight parameter type.
pub trait SupportedParameterType {}
/// This is a marker trait that is used to indicate that a type is a valid Hyperlight return type.
pub trait SupportedReturnType {}

/// This trait allows us to get the HyperlightType for a type at run time
pub trait SupportedParameterAndReturnTypesInfo {
    /// Get the SupportedParameterAndReturnTypes for a type
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes;
}

// We can then implement these traits for each type that support as a parameter or return type.

impl SupportedParameterType for u32 {}
impl SupportedParameterType for String {}
impl SupportedParameterType for i32 {}
impl SupportedParameterType for i64 {}
impl SupportedParameterType for u64 {}
impl SupportedParameterType for bool {}
impl SupportedParameterType for Vec<u8> {}
impl SupportedParameterType for *mut std::ffi::c_void {}
// etc

impl SupportedReturnType for u32 {}
impl SupportedReturnType for () {}
impl SupportedReturnType for String {}
impl SupportedReturnType for i32 {}
impl SupportedReturnType for i64 {}
impl SupportedReturnType for u64 {}
impl SupportedReturnType for bool {}
impl SupportedReturnType for Vec<u8> {}
impl SupportedReturnType for *mut std::ffi::c_void {}
// etc

// and we can implement HyperlightReturnandParamTypeInfo so we can get the actual type when we register or dispatch a function call.
// e.g. in register_host_function below we can interogate the HyperlightReturnandParamTypeInfo to determine the type of the parameter or return value
// validate that it is correct for the expected host function

impl SupportedParameterAndReturnTypesInfo for u32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::UInt
    }
}

impl SupportedParameterAndReturnTypesInfo for String {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::String
    }
}

impl SupportedParameterAndReturnTypesInfo for () {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Void
    }
}

impl SupportedParameterAndReturnTypesInfo for i32 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Int
    }
}

impl SupportedParameterAndReturnTypesInfo for i64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Long
    }
}

impl SupportedParameterAndReturnTypesInfo for u64 {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ULong
    }
}

impl SupportedParameterAndReturnTypesInfo for bool {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::Bool
    }
}

impl SupportedParameterAndReturnTypesInfo for Vec<u8> {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::ByteArray
    }
}

impl SupportedParameterAndReturnTypesInfo for std::ffi::c_void {
    fn get_hyperlight_type() -> SupportedParameterAndReturnTypes {
        SupportedParameterAndReturnTypes::IntPtr
    }
}

// We can then define a structs that represents a host function with different numbers of arguments and return types
// And constrain the types such that they can only be used with valid Hyperlight parameter and return types
// This gives us compile time checking that the types are valid for Hyperlight

// Note that we are using Anyhow Result here at the moment.

/// A Hyperlight host function that takes no arguments and returns a result
pub type HostFunctionWithNoArgsType<'a, R> = Rc<RefCell<dyn FnMut() -> Result<R> + 'a>>;

#[allow(unused)]
/// A Hyperlight host function that takes no arguments and returns a result
pub struct HostFunctionWithNoArgs<'a, R>
where
    R: SupportedReturnType + SupportedParameterAndReturnTypesInfo,
{
    /// A Hyperlight host function that takes no arguments and returns a result
    pub func: HostFunctionWithNoArgsType<'a, R>,
}

/// A Hyperlight host function that takes 1 argument and returns a result
pub type HostFunctionWithOneArgType<'a, R, P1> = Rc<RefCell<dyn FnMut(P1) -> Result<R> + 'a>>;

/// A Hyperlight host function that takes 1 argument and returns a result
pub struct HostFunctionWithOneArg<'a, R, P1>
where
    R: SupportedReturnType + SupportedParameterAndReturnTypesInfo,
    P1: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
{
    /// A Hyperlight host function that takes 1 argument and returns a result
    pub func: HostFunctionWithOneArgType<'a, R, P1>,
}

/// A Hyperlight host function that takes 2 arguments and returns a result
pub type HostFunctionWithTwoArgsType<'a, R, P1, P2> =
    Rc<RefCell<dyn FnMut(P1, P2) -> Result<R> + 'a>>;

#[allow(unused)]
pub(crate) struct HostFunctionWithTwoArgs<'a, R, P1, P2>
where
    R: SupportedReturnType + SupportedParameterAndReturnTypesInfo,
    P1: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P2: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
{
    /// A Hyperlight host function that takes 2 arguments and returns a result
    pub(crate) func: HostFunctionWithTwoArgsType<'a, R, P1, P2>,
}

/// A Hyperlight host function that takes 3 arguments and returns a result
pub type HostFunctionWithThreeArgsType<'a, R, P1, P2, P3> =
    Rc<RefCell<dyn FnMut(P1, P2, P3) -> Result<R> + 'a>>;

#[allow(unused)]
pub(crate) struct HostFunctionWithThreeArgs<'a, R, P1, P2, P3>
where
    R: SupportedReturnType + SupportedParameterAndReturnTypesInfo,
    P1: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P2: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P3: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
{
    /// A Hyperlight host function that takes 3 arguments and returns a result
    pub(crate) func: HostFunctionWithThreeArgsType<'a, R, P1, P2, P3>,
}

/// A Hyperlight host function that takes 4 arguments and returns a result
pub type HostFunctionWithFourArgsType<'a, R, P1, P2, P3, P4> =
    Rc<RefCell<dyn FnMut(P1, P2, P3, P4) -> Result<R> + 'a>>;

#[allow(unused)]
/// A Hyperlight host function that takes 4 arguments and returns a result
pub(crate) struct HostFunctionWithFourArgs<'a, R, P1, P2, P3, P4>
where
    R: SupportedReturnType + SupportedParameterAndReturnTypesInfo,
    P1: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P2: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P3: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P4: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
{
    /// A Hyperlight host function that takes 4 arguments and returns a result
    pub(crate) func: HostFunctionWithFourArgsType<'a, R, P1, P2, P3, P4>,
}

/// A Hyperlight host function that takes 5 arguments and returns a result
pub type HostFunctionWithFiveArgsType<'a, R, P1, P2, P3, P4, P5> =
    Rc<RefCell<dyn FnMut(P1, P2, P3, P4, P5) -> Result<R> + 'a>>;

#[allow(unused)]
/// A Hyperlight host function that takes 5 arguments and returns a result
pub(crate) struct HostFunctionWithFiveArgs<'a, R, P1, P2, P3, P4, P5>
where
    R: SupportedReturnType + SupportedParameterAndReturnTypesInfo,
    P1: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P2: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P3: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P4: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
    P5: SupportedParameterType + SupportedParameterAndReturnTypesInfo,
{
    /// A Hyperlight host function that takes 5 arguments and returns a result
    pub(crate) func: HostFunctionWithFiveArgsType<'a, R, P1, P2, P3, P4, P5>,
}

// this would mean that register_host_function would need to accept any of the above structs , I think this can be solved via the implementation of another trait function
// that checks which of the concrete types the trait is and deals with it appropriately the example below checks for the HostFunctionWithOneArg type so it can be used in a test
// but it illustrates the idea
// This could be an enormous function because of all the different combinations of parameter and return types but we should be able to generate the code for this
// perf should not be too much of a concern as we only need to do this once per host function registration dispatching a call would look up the type info regsitered
// so there would be minimal overhead at call dispatch.

// this is a simple version of a dynamic type check its only used in a test below to illustrate the idea
// I dont yet know how to make this work when a closure is passed as dyn any seems to require a static lifetime
#[allow(unused)]
fn validate_concrete_type(t: &dyn Any) -> Result<()> {
    if let Some(_f) = t.downcast_ref::<HostFunctionWithOneArg<'_, (), String>>() {
        println!(
            "HostFunctionWithOneArg<(),String>: TypeId {:?}",
            t.type_id()
        );
        Ok(())
    } else {
        Err(anyhow!(
            "Not a HostFunctionWithOneArg taking a String parameter and returning a ()"
        ))
    }
}

/// Sandboxes are the primary mechanism to interact with VM partitions.
///
/// Prior to initializing a Sandbox, the caller must register all host functions
/// onto an UninitializedSandbox. Once all host functions have been registered,
/// the UninitializedSandbox can be initialized into a Sandbox through the
/// `initialize` method.
pub struct UnintializedSandbox<'a> {
    // The writer to use for print requests from the guest.
    //  writer_func: PrintOutputFunctionPointer<'a>,
    writer_func: HostFunctionWithOneArg<'a, (), String>, // DAN:TODO replace w/ map of host functions and register writer_func in it by default
    /// The map of host function names to their corresponding
    /// HostMethodInfo.
    map_host_function_names_to_method_info: HashMap<String, HostMethodInfo>,
    // The memory manager for the sandbox.
    mem_mgr: SandboxMemoryManager,
    stack_guard: [u8; STACK_COOKIE_LEN],
}

/// The primary mechanism to interact with VM partitions that
/// run Hyperlight Sandboxes.
///
/// A Hyperlight Sandbox is a specialized VM environment
/// intended specifically for running Hyperlight guest processes.
#[allow(unused)]
pub struct Sandbox {
    // (DAN:TODO) Add field for the host_functions map
    mem_mgr: SandboxMemoryManager,
    stack_guard: [u8; STACK_COOKIE_LEN],
}

impl<'a> UnintializedSandbox<'a> {
    /// Create a new sandbox configured to run the binary at path
    /// `bin_path`.
    pub fn new(
        bin_path: String,
        cfg: Option<SandboxMemoryConfiguration>,
        writer_func: Option<HostFunctionWithOneArg<'a, (), String>>,
        sandbox_run_options: Option<SandboxRunOptions>,
    ) -> Result<Self> {
        // Make sure the binary exists

        let path = Path::new(&bin_path).canonicalize()?;
        path.try_exists()?;

        let sandbox_run_options =
            sandbox_run_options.unwrap_or(SandboxRunOptions::RUN_IN_HYPERVISOR);

        let run_from_process_memory = sandbox_run_options
            .contains(SandboxRunOptions::RUN_IN_PROCESS)
            || sandbox_run_options.contains(SandboxRunOptions::RUN_FROM_GUEST_BINARY);
        let run_from_guest_binary =
            sandbox_run_options.contains(SandboxRunOptions::RUN_FROM_GUEST_BINARY);

        if run_from_guest_binary
            && sandbox_run_options.contains(SandboxRunOptions::RECYCLE_AFTER_RUN)
        {
            anyhow::bail!("Recycle after run at is not supported when running from guest binary.");
        }

        let mem_cfg = cfg.unwrap_or_default();
        let mut mem_mgr = UnintializedSandbox::load_guest_binary(
            mem_cfg,
            &bin_path,
            run_from_process_memory,
            run_from_guest_binary,
        )?;
        let stack_guard = Self::create_stack_guard();
        mem_mgr.set_stack_guard(&stack_guard)?;

        // The default writer function is to write to stdout with green text

        let writer_func: HostFunctionWithOneArg<'a, (), String> =
            writer_func.unwrap_or(HostFunctionWithOneArg {
                func: Rc::new(RefCell::new(|s: String| -> Result<()> {
                    match atty::is(atty::Stream::Stdout) {
                        false => {
                            stdout().write_all(s.as_bytes())?;
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
                }))
                .clone(),
            });

        let sandbox = Self {
            writer_func,
            mem_mgr,
            map_host_function_names_to_method_info: HashMap::new(),
            stack_guard,
        };

        // TODO: Register the host print function

        Ok(sandbox)
    }

    fn create_stack_guard() -> [u8; STACK_COOKIE_LEN] {
        rand::random::<[u8; STACK_COOKIE_LEN]>()
    }

    /// Check the stack guard against the stack guard cookie stored
    /// within `self`. Return `Ok(true)` if the guard cookie could
    /// be found and it matched `self.stack_guard`, `Ok(false)` if
    /// if could be found and did not match `self.stack_guard`, and
    /// `Err` if it could not be found or there was some other error.
    ///
    /// TODO: remove the dead code annotation after this is hooked up in
    /// https://github.com/deislabs/hyperlight/pull/727
    #[allow(dead_code)]
    fn check_stack_guard(&self) -> Result<bool> {
        self.mem_mgr.check_stack_guard(self.stack_guard)
    }

    /// Set up the appropriate hypervisor for the platform.
    ///
    /// this function is used to prevent clippy from complaining
    /// the 'mgr' param is unused on windows builds. this function and the
    /// function of the same name on linux builds will merge when we
    /// have a complete WHP implementation in Rust.
    ///
    /// TODO: remove this dead_code annotation after it's hooked up in
    /// https://github.com/deislabs/hyperlight/pull/727/files, and merge with
    /// linux version of this function
    #[allow(dead_code)]
    #[cfg(target_os = "windows")]
    fn set_up_hypervisor_partition(_: &mut SandboxMemoryManager) -> Result<Box<dyn Hypervisor>> {
        bail!("Hyperlight does not yet support Windows");
    }

    /// Set up the appropriate hypervisor for the platform
    ///
    /// TODO: remove this dead_code annotation after it's hooked up in
    /// https://github.com/deislabs/hyperlight/pull/727/files,
    /// and merge with the windows version of this function
    #[allow(dead_code)]
    #[cfg(target_os = "linux")]
    fn set_up_hypervisor_partition(mgr: &mut SandboxMemoryManager) -> Result<Box<dyn Hypervisor>> {
        let mem_size = u64::try_from(mgr.shared_mem.mem_size())?;
        let rsp = mgr.set_up_hypervisor_partition(mem_size)?;
        let base_addr = SandboxMemoryLayout::BASE_ADDRESS;
        let pml4_addr = base_addr + SandboxMemoryLayout::PML4_OFFSET;
        let entrypoint = {
            let load_addr = mgr.load_addr.clone();
            let load_offset_u64 =
                u64::from(load_addr) - u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?;
            let total_offset = Offset::from(load_offset_u64) + mgr.entrypoint_offset;
            GuestPtr::try_from(total_offset)
        }?;
        if hyperv_linux::is_hypervisor_present(REQUIRE_STABLE_API)? {
            let guest_pfn = u64::try_from(SandboxMemoryLayout::BASE_ADDRESS >> 12)?;
            let host_addr = u64::try_from(mgr.shared_mem.base_addr())?;
            let addrs = HypervisorAddrs {
                entrypoint: entrypoint.absolute()?,
                guest_pfn,
                host_addr,
                mem_size,
            };
            let hv = HypervLinuxDriver::new(REQUIRE_STABLE_API, &addrs)?;
            Ok(Box::new(hv))
        } else if kvm::is_hypervisor_present().is_ok() {
            let host_addr = u64::try_from(mgr.shared_mem.base_addr())?;
            let hv = KVMDriver::new(
                host_addr,
                u64::try_from(pml4_addr)?,
                mem_size,
                entrypoint.absolute()?,
                rsp,
            )?;
            Ok(Box::new(hv))
        } else {
            bail!("Linux platform detected, but neither KVM nor Linux HyperV detected")
        }
    }

    /// Registers a host function onto map of host functions.
    ///
    /// Example usage:
    /// ```
    /// use hyperlight_host::guest_interface_glue::register_host_function;
    /// use hyperlight_host::guest::host_function_definition::{HostFunctionDefinition, ParamValueType, ReturnValueType};
    /// use hyperlight_host::guest_interface_glue::SupportedParameterAndReturnTypes;
    ///
    /// fn add(args: &[SupportedParameterAndReturnValues]) -> Result<SupportedParameterAndReturnValues> {
    ///    let a = match &args[0] {
    ///             SupportedParameterAndReturnValues::Int(a) => *a,
    ///             _ => return Err(anyhow!("Invalid type for a")),
    ///     };
    ///     let b = match &args[1] {
    ///             SupportedParameterAndReturnValues::Int(b) => *b,
    ///             _ => return Err(anyhow!("Invalid type for b")),
    ///     };
    ///     Ok(SupportedParameterAndReturnValues::Int(a + b))
    /// }
    ///
    ///
    /// fn main() {
    ///    let function = HostMethodInfo {
    ///       host_function_definition: HostFunctionDefinition {
    ///         function_name: "add".to_string(),
    ///         parameters: vec![ ParamValueType::Int, ParamValueType::Int ],
    ///         return_type: ReturnValueType::Int,
    ///       },
    ///       function_pointer: add,
    ///     };
    ///    register_host_function(function);
    /// ```
    ///
    pub fn register_host_function(&mut self, function: HostMethodInfo) -> Result<()> {
        // (DAN:TODO) This will be completely refactored.
        let name = function.host_function_definition.function_name.to_string();
        let map = &mut self.map_host_function_names_to_method_info;

        // If already exists, replace
        if map.contains_key(&name) {
            // (DAN:TODO): log warning equiv. to "HyperlightLogger.LogWarning($"Updating MethodInfo for ${methodInfo.&Name} - there are multiple host methods with the same name.", GetType().Name);"
            map.remove(&name);
        }

        map.insert(name, function);
        Ok(())
    }

    /// Calls a host function by name.
    ///
    /// Example usage:
    /// ```
    /// // [...]
    /// match call_host_function("add", &vec![SupportedParameterAndReturnValues::Int(1), SupportedParameterAndReturnValues::Int(2)]) {
    ///     Ok(SupportedParameterAndReturnValues::Int(result)) => println!("Result: {}", result),
    ///     _ => println!("Invalid return type"),
    /// }
    /// // [...]
    /// ```
    ///
    pub fn call_host_function(
        // (DAN:TODO) This will be completely refactored.
        &mut self,
        function_name: &str,
        args: &[SupportedParameterAndReturnValues],
    ) -> Result<SupportedParameterAndReturnValues> {
        let map = &mut self.map_host_function_names_to_method_info;

        let host_function = match map.get_mut(function_name) {
            Some(host_function) => host_function,
            None => return Err(anyhow!("Host function not found")),
        };

        (host_function.function_pointer)(args)
    }

    /// Determine whether a suitable hypervisor is available to run
    /// this sandbox.
    ///
    /// Returns `Ok` with a boolean if it could be determined whether
    /// an appropriate hypervisor is available, and `Err` otherwise.
    pub fn is_hypervisor_present(&self) -> Result<bool> {
        // TODO: implement
        Ok(true)
    }

    /// TODO: This should be removed once we have a proper Sandbox with C API that provides all functionaliy
    /// It only exists to keep the C# code working for now
    ///
    pub(crate) fn get_mem_mgr(&self) -> SandboxMemoryManager {
        self.mem_mgr.clone()
    }

    /// Call the entry point inside this `Sandbox`
    pub(crate) unsafe fn call_entry_point(
        &self,
        peb_address: RawPtr,
        seed: u64,
        page_size: u32,
    ) -> Result<()> {
        type EntryPoint = extern "C" fn(i64, u64, u32) -> i32;
        let entry_point: EntryPoint = {
            let addr = {
                let offset = self.mem_mgr.entrypoint_offset;
                self.mem_mgr.load_addr.clone().add(offset)
            };

            let fn_location = u64::from(addr) as *const c_void;
            std::mem::transmute(fn_location)
        };
        let peb_i64 = i64::try_from(u64::from(peb_address))?;
        entry_point(peb_i64, seed, page_size);
        Ok(())
    }

    /// Load the file at `bin_path_str` into a PE file, then attempt to
    /// load the PE file into a `SandboxMemoryManager` and return it.
    ///
    /// If `run_from_guest_binary` is passed as `true`, and this code is
    /// running on windows, this function will call
    /// `SandboxMemoryManager::load_guest_binary_using_load_library` to
    /// create the new `SandboxMemoryManager`. If `run_from_guest_binary` is
    /// passed as `true` and we're not running on windows, this function will
    /// return an `Err`. Otherwise, if `run_from_guest_binary` is passed
    /// as `false`, this function calls `SandboxMemoryManager::load_guest_binary_into_memory`.
    ///
    fn load_guest_binary(
        mem_cfg: SandboxMemoryConfiguration,
        bin_path_str: &str,
        run_from_process_memory: bool,
        run_from_guest_binary: bool,
    ) -> Result<SandboxMemoryManager> {
        let mut pe_info = PEInfo::from_file(bin_path_str)?;
        if run_from_guest_binary {
            SandboxMemoryManager::load_guest_binary_using_load_library(
                mem_cfg,
                bin_path_str,
                &mut pe_info,
                run_from_process_memory,
            )
            // TODO: This produces the wrong error message on Linux and is possibly obsfucating the real error on Windows
            .map_err(|_| {
                let err_msg =
                    "Only one instance of Sandbox is allowed when running from guest binary";
                anyhow!(err_msg)
            })
        } else {
            SandboxMemoryManager::load_guest_binary_into_memory(
                mem_cfg,
                &mut pe_info,
                run_from_process_memory,
            )
        }
    }
    #[allow(unused)]
    pub(crate) fn handle_outb(&self, port: u16, byte: u8) -> Result<()> {
        match port.into() {
            OutBAction::Log => outb_log(&self.mem_mgr),
            OutBAction::CallFunction => {
                // TODO
                todo!();
            }
            OutBAction::Abort => {
                // TODO
                todo!();
            }
            _ => {
                // TODO
                todo!();
            }
        }
    }

    // TODO: function is temporary to allow the testing of C API providing a Print function remove this when we have a proper Sandbox with C API
    pub(crate) fn host_print(&mut self, msg: String) -> Result<()> {
        // The try_borrow_mut is not always going to be needed here.
        // Ideally we would figure if the writer_func is an FnMut or if its one of its subtraits (in which case we would not need to borrow_mut)

        (self.writer_func.func.try_borrow_mut()?)(msg)
    }

    /// Check for a guest error and return an `Err` if one was found,
    /// and `Ok` if one was not found.
    /// TODO: remove this when we hook it up to the rest of the
    /// sandbox in https://github.com/deislabs/hyperlight/pull/727
    #[allow(dead_code)]
    fn check_for_guest_error(&self) -> Result<()> {
        let guest_err = self.mem_mgr.get_guest_error()?;
        match guest_err.code {
            ErrorCode::NoError => Ok(()),
            ErrorCode::OutbError => match self.mem_mgr.get_host_error()? {
                Some(host_err) => bail!("[OutB Error] {:?}: {:?}", guest_err.code, host_err),
                None => Ok(()),
            },
            ErrorCode::StackOverflow => {
                let err_msg = format!(
                    "[Stack Overflow] Guest Error: {:?}: {}",
                    guest_err.code, guest_err.message
                );
                error!("{}", err_msg);
                bail!(err_msg);
            }
            _ => {
                let err_msg = format!("Guest Error: {:?}: {}", guest_err.code, guest_err.message);
                error!("{}", err_msg);
                bail!(err_msg);
            }
        }
    }

    /// Initialize the `Sandbox` from an `UninitializedSandbox`.
    /// Receives a callback function to be called during initialization.
    #[allow(unused)]
    fn initialize<F: Fn(&mut Sandbox) -> Result<()>>(&mut self, callback: F) -> Result<Sandbox> {
        let mut sbox = Sandbox {
            mem_mgr: self.mem_mgr.clone(),
            stack_guard: self.stack_guard,
        };
        callback(&mut sbox)?;

        Ok(sbox)
    }
}

impl Sandbox {
    // (DAN:TODO) Add function to register new or delete host functions. This should return an `UninitializedSandbox`.
}

fn outb_log(mgr: &SandboxMemoryManager) -> Result<()> {
    let log_data: GuestLogData = mgr.read_guest_log_data()?;
    let log_level = &log_data.level;
    let message = format!(
        "{} [{}:{}] {}, {}\n",
        log_data.source, log_data.source_file, log_data.line, log_data.caller, log_data.message
    );
    match log_level {
        LogLevel::Trace => trace!("{}", message),
        LogLevel::Debug => debug!("{}", message),
        LogLevel::Information => info!("{}", message),
        LogLevel::Warning => warn!("{}", message),
        LogLevel::Error => error!("{}", message),
        LogLevel::Critical => error!("[CRITICAL] {}", message),
        // Do nothing if the log level is set to none
        LogLevel::None => (),
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{outb_log, validate_concrete_type, HostFunctionWithOneArg, UnintializedSandbox};
    use crate::{
        guest::{guest_log_data::GuestLogData, log_level::LogLevel},
        mem::{config::SandboxMemoryConfiguration, mgr::SandboxMemoryManager},
        sandbox_run_options::SandboxRunOptions,
        testing::{logger::LOGGER, simple_guest_path, simple_guest_pe_info},
    };
    use anyhow::Result;
    use log::{set_logger, set_max_level, Level};
    use std::io::{Read, Write};
    use std::{cell::RefCell, rc::Rc};
    use tempfile::NamedTempFile;
    #[test]

    fn test_new_sandbox() {
        // Guest Binary exists at path

        let binary_path = simple_guest_path().unwrap();
        let sandbox = UnintializedSandbox::new(binary_path.clone(), None, None, None);
        assert!(sandbox.is_ok());

        // Guest Binary does not exist at path

        let binary_path_does_not_exist = binary_path.trim_end_matches(".exe").to_string();
        let sandbox = UnintializedSandbox::new(binary_path_does_not_exist, None, None, None);
        assert!(sandbox.is_err());

        // Non default memory configuration

        let cfg = SandboxMemoryConfiguration::new(
            0x1000,
            0x1000,
            0x1000,
            0x1000,
            0x1000,
            Some(0x1000),
            Some(0x1000),
        );

        let sandbox = UnintializedSandbox::new(binary_path.clone(), Some(cfg), None, None);
        assert!(sandbox.is_ok());

        // Invalid sandbox_run_options

        let sandbox_run_options =
            SandboxRunOptions::RUN_FROM_GUEST_BINARY | SandboxRunOptions::RECYCLE_AFTER_RUN;

        let sandbox = UnintializedSandbox::new(binary_path, None, None, Some(sandbox_run_options));
        assert!(sandbox.is_err());
    }

    #[test]
    fn test_load_guest_binary_manual() {
        let cfg = SandboxMemoryConfiguration::default();

        let simple_guest_path = simple_guest_path().unwrap();
        let mgr =
            UnintializedSandbox::load_guest_binary(cfg, simple_guest_path.as_str(), false, false)
                .unwrap();
        assert_eq!(cfg, mgr.mem_cfg);
    }

    #[test]
    fn test_load_guest_binary_load_lib() {
        let cfg = SandboxMemoryConfiguration::default();
        let simple_guest_path = simple_guest_path().unwrap();
        let mgr_res =
            UnintializedSandbox::load_guest_binary(cfg, simple_guest_path.as_str(), true, true);
        #[cfg(target_os = "linux")]
        {
            assert!(mgr_res.is_err())
        }
        #[cfg(target_os = "windows")]
        {
            let _ = mgr_res.unwrap();
        }
    }
    #[test]
    fn test_host_print() {
        // writer as a FnMut closure mutating a captured variable and then trying to access the captured variable
        // after the Sandbox instance has been dropped
        // this example is fairly contrived but we should still support such an approach.

        let mut received_msg = String::new();

        let writer = |msg| {
            received_msg = msg;
            Ok(())
        };

        let writer_func = Rc::new(RefCell::new(writer));

        let mut sandbox = UnintializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            Some(HostFunctionWithOneArg {
                func: writer_func.clone(),
            }),
            None,
        )
        .expect("Failed to create sandbox");

        sandbox.host_print("test".to_string()).unwrap();

        drop(sandbox);

        assert_eq!(&received_msg, "test");

        // There may be cases where a mutable reference to the captured variable is not required to be used outside the closue
        // e.g. if the function is writing to a file or a socket etc.

        // writer as a FnMut closure mutating a captured variable but not trying to access the captured variable

        // This seems more realistic as the client is creating a file to be written to in the closure
        // and then accessing the file a different handle.
        // The problem is that captured_file still needs static lifetime so even though we can access the data through the second file handle
        // this still does not work as the captured_file is dropped at the end of the function

        let mut captured_file = NamedTempFile::new().unwrap();
        let mut file = captured_file.reopen().unwrap();

        let writer = |msg: String| -> Result<()> {
            captured_file.write_all(msg.as_bytes()).unwrap();
            Ok(())
        };

        let writer_func = Rc::new(RefCell::new(writer));

        let mut sandbox = UnintializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            Some(HostFunctionWithOneArg {
                func: writer_func.clone(),
            }),
            None,
        )
        .expect("Failed to create sandbox");

        sandbox.host_print("test2".to_string()).unwrap();

        let mut buffer = String::new();
        file.read_to_string(&mut buffer).unwrap();
        assert_eq!(buffer, "test2");

        // writer as a function

        fn fn_writer(msg: String) -> Result<()> {
            assert_eq!(msg, "test2");
            Ok(())
        }

        let writer_func = Rc::new(RefCell::new(fn_writer));
        let mut sandbox = UnintializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            Some(HostFunctionWithOneArg { func: writer_func }),
            None,
        )
        .expect("Failed to create sandbox");

        sandbox.host_print("test2".to_string()).unwrap();

        // writer as a method

        let mut test_host_print = TestHostPrint::new();

        // create a closure over the struct method

        let writer_closure = |s| test_host_print.write(s);

        let writer_method = Rc::new(RefCell::new(writer_closure));

        let mut sandbox = UnintializedSandbox::new(
            simple_guest_path().expect("Guest Binary Missing"),
            None,
            Some(HostFunctionWithOneArg {
                func: writer_method.clone(),
            }),
            None,
        )
        .expect("Failed to create sandbox");

        sandbox.host_print("test3".to_string()).unwrap();

        // Simulate dynamic type checking
        // Note : Not yet able to get this to work with closures

        let writer_func = Rc::new(RefCell::new(fn_writer));

        let host_function_with_one_arg = HostFunctionWithOneArg { func: writer_func };

        let result = validate_concrete_type(&host_function_with_one_arg);

        assert!(result.is_ok());

        // TODO: Test with stdout
    }

    struct TestHostPrint {}

    impl TestHostPrint {
        fn new() -> Self {
            TestHostPrint {}
        }

        fn write(&mut self, msg: String) -> Result<()> {
            assert_eq!(msg, "test3");
            Ok(())
        }
    }

    fn new_guest_log_data(level: LogLevel) -> GuestLogData {
        GuestLogData::new(
            "test log".to_string(),
            "test source".to_string(),
            level,
            "test caller".to_string(),
            "test source file".to_string(),
            123,
        )
    }

    #[test]
    fn test_outb_log() {
        let new_mgr = || {
            let mut pe_info = simple_guest_pe_info().unwrap();
            SandboxMemoryManager::load_guest_binary_into_memory(
                SandboxMemoryConfiguration::default(),
                &mut pe_info,
                false,
            )
            .unwrap()
        };
        {
            // We have not set a logger and there is no guest log data
            // in memory, so expect a log operation to fail
            let mgr = new_mgr();
            assert!(outb_log(&mgr).is_err());
        }
        {
            // Write a log message so outb_log will succeed. Since there is
            // no logger set with set_logger, expect logs to be no-ops
            let mut mgr = new_mgr();
            let layout = mgr.layout;
            let log_msg = new_guest_log_data(LogLevel::Information);

            log_msg
                .write_to_memory(mgr.get_shared_mem_mut(), &layout)
                .unwrap();
            assert!(outb_log(&mgr).is_ok());
            assert_eq!(0, LOGGER.num_log_calls());
        }
        {
            // now, test logging
            let mut mgr = new_mgr();
            {
                // set up the logger and set the log level to the maximum
                // possible (Trace) to ensure we're able to test all
                // the possible branches of the match in outb_log
                set_logger(&LOGGER).unwrap();
                set_max_level(log::LevelFilter::Trace);
            }
            let levels = vec![
                LogLevel::Trace,
                LogLevel::Debug,
                LogLevel::Information,
                LogLevel::Warning,
                LogLevel::Error,
                LogLevel::Critical,
                LogLevel::None,
            ];
            for (idx, level) in levels.iter().enumerate() {
                let layout = mgr.layout;
                let log_data = new_guest_log_data(level.clone());
                log_data
                    .write_to_memory(mgr.get_shared_mem_mut(), &layout)
                    .unwrap();
                outb_log(&mgr).unwrap();
                let num_calls = LOGGER.num_log_calls();
                if level.clone() != LogLevel::None {
                    assert_eq!(
                        idx + 1,
                        num_calls,
                        "log call did not occur for level {:?}",
                        level.clone()
                    );
                }
                let last_log = LOGGER.get_log_call(num_calls - 1).unwrap();
                match (level, last_log.level) {
                    (LogLevel::Trace, Level::Trace) => (),
                    (LogLevel::Debug, Level::Debug) => (),
                    (LogLevel::Information, Level::Info) => (),
                    (LogLevel::Warning, Level::Warn) => (),
                    (LogLevel::Error, Level::Error) => (),
                    (LogLevel::Critical, Level::Error) => (),
                    // If someone logged with "None", we don't
                    // expect any actual log record. this case
                    // is here to indicate we don't want to
                    // match None to any actual log record, and
                    // we don't want to fall through to the next
                    // case
                    (LogLevel::None, _) => (),
                    (other_log_level, other_level) => panic!(
                        "Invalid LogLevel / Level pair: ({:?}, {:?})",
                        other_log_level, other_level
                    ),
                };
            }
        }
    }

    #[test]
    fn test_stack_guard() {
        let simple_guest_path = simple_guest_path().unwrap();
        let sbox = UnintializedSandbox::new(simple_guest_path, None, None, None).unwrap();
        let res = sbox.check_stack_guard();
        assert!(res.is_ok(), "Sandbox::check_stack_guard returned an error");
        assert!(res.unwrap(), "Sandbox::check_stack_guard returned false");
    }
}
