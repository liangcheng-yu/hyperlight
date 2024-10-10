use core::ffi::c_void;
use core::time::Duration;
use std::ops::Add;
use std::sync::{Arc, Mutex};

use rand::Rng;
use tracing::{instrument, Span};

use super::leaked_outb::LeakedOutBWrapper;
use crate::func::exports::get_os_page_size;
use crate::hypervisor::handlers::OutBHandlerWrapper;
use crate::hypervisor::hypervisor_handler::{
    HvHandlerConfig, HypervisorHandler, HypervisorHandlerAction,
};
#[cfg(target_os = "linux")]
use crate::log_then_return;
use crate::mem::mgr::SandboxMemoryManager;
use crate::mem::ptr::RawPtr;
use crate::mem::shared_mem::GuestSharedMemory;
#[cfg(windows)]
use crate::mem::shared_mem::SharedMemory;
use crate::sandbox::host_funcs::HostFuncsWrapper;
use crate::sandbox::mem_access::mem_access_handler_wrapper;
use crate::sandbox::outb::outb_handler_wrapper;
use crate::sandbox::{HostSharedMemory, MemMgrWrapper};
use crate::sandbox_state::sandbox::Sandbox;
use crate::{new_error, MultiUseSandbox, Result, SingleUseSandbox, UninitializedSandbox};

#[derive(Clone)]
pub(crate) enum ExecutionMode<'a> {
    #[allow(dead_code)]
    InProc(LeakedOutBWrapper<'a>),
    InHypervisor(HypervisorHandler),
}

/// The implementation for evolving `UninitializedSandbox`es to
/// `Sandbox`es.
///
/// Note that `cb_opt`'s type has been carefully considered.
/// Particularly, it's not using a constrained generic to define
/// the type of the callback because if it did, you'd have to provide
/// type hints to the compiler if you want to pass `None` to the function.
/// With this type signature, you can pass `None` without having to do that.
///
/// If this doesn't make sense, and you want to change this type,
/// please reach out to a Hyperlight developer before making the change.
#[instrument(err(Debug), skip_all, , parent = Span::current(), level = "Trace")]
fn evolve_impl<'a, TransformFunc, ResSandbox: Sandbox>(
    u_sbox: UninitializedSandbox,
    transform: TransformFunc,
) -> Result<ResSandbox>
where
    TransformFunc: Fn(
        Arc<Mutex<HostFuncsWrapper>>,
        MemMgrWrapper<HostSharedMemory>,
        ExecutionMode<'a>,
    ) -> Result<ResSandbox>,
{
    let run_from_proc_mem = u_sbox.run_from_process_memory;

    let (hshm, gshm) = u_sbox.mgr.build();

    let execution_mode = if run_from_proc_mem {
        let outb_wrapper = outb_handler_wrapper(hshm.clone(), u_sbox.host_funcs.clone());
        let leaked_outb = evolve_in_proc(hshm.clone(), gshm, outb_wrapper)?;
        ExecutionMode::InProc(leaked_outb)
    } else {
        let mut hv_handler = hv_init(
            &hshm,
            gshm,
            u_sbox.host_funcs.clone(),
            u_sbox.max_initialization_time,
            u_sbox.max_execution_time,
            u_sbox.max_wait_for_cancellation,
        )?;

        {
            let dispatch_function_addr = hshm.as_ref().get_pointer_to_dispatch_function()?;
            assert_ne!(dispatch_function_addr, 0);
            hv_handler.set_dispatch_function_addr(RawPtr::from(dispatch_function_addr))?;
        }

        ExecutionMode::InHypervisor(hv_handler)
    };

    transform(u_sbox.host_funcs, hshm, execution_mode)
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
pub(super) fn evolve_impl_multi_use<'a>(
    u_sbox: UninitializedSandbox,
) -> Result<MultiUseSandbox<'a>> {
    evolve_impl(u_sbox, |hf, mut hshm, execution_mode| {
        {
            hshm.as_mut().push_state()?;
        }
        Ok(MultiUseSandbox::from_uninit(hf, hshm, execution_mode))
    })
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
pub(super) fn evolve_impl_single_use<'a>(
    u_sbox: UninitializedSandbox,
) -> Result<SingleUseSandbox<'a>> {
    evolve_impl(u_sbox, |_hf, hshm, execution_mode| {
        // Its intentional not to snapshot state here. This is because
        // single use sandboxes are not reusable and so there is no need
        // to snapshot state as they cannot be devolved back to an uninitialized sandbox.
        Ok(SingleUseSandbox::from_uninit(hshm, execution_mode))
    })
}

/// Call the entry point inside this `Sandbox` and return `Ok(())` if
/// the entry point returned successfully. This function only applies to
/// sandboxes with in-process mode turned on (e.g.
/// `SandboxRunOptions::RunInProcess` passed as run options to the
/// `UninitializedSandbox::new` function). If in-process mode is not
/// turned on this function does nothing and immediately returns an `Err`.
///
/// # Safety
///
/// The given `peb_address` parameter must be an address in the guest
/// memory corresponding to the start of the process
/// environment block (PEB). If running with in-process mode, it must
/// be an address into the host memory that points to the PEB.
///
/// Additionally, `page_size` must correspond to the operating system's
/// chosen size of a virtual memory page.
#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
pub(super) unsafe fn call_entry_point(
    mgr: &SandboxMemoryManager<GuestSharedMemory>,
    peb_address: RawPtr,
    seed: u64,
    page_size: u32,
) -> Result<()> {
    type EntryPoint = extern "C" fn(i64, u64, u32, u32) -> i32;
    let entry_point: EntryPoint = {
        let addr = {
            let offset = mgr.entrypoint_offset;
            mgr.load_addr.clone().add(offset)
        };

        let fn_location = u64::from(addr) as *const c_void;
        unsafe { std::mem::transmute(fn_location) }
    };
    let peb_i64 = i64::try_from(u64::from(peb_address))?;
    let max_log_level = log::max_level() as u32;
    entry_point(peb_i64, seed, page_size, max_log_level);
    Ok(())
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
fn evolve_in_proc<'a>(
    mut _hshm: MemMgrWrapper<HostSharedMemory>,
    mut _gshm: SandboxMemoryManager<GuestSharedMemory>,
    outb_hdl: OutBHandlerWrapper,
) -> Result<LeakedOutBWrapper<'a>> {
    #[cfg(target_os = "linux")]
    {
        // Note from old C# implementation of this function:
        //
        // This code is unstable, it causes segmentation faults so for now we
        // are throwing an exception if we try to run in process in Linux.
        // I think this is due to the fact that the guest binary is built for
        // windows x64 compilation for windows uses fastcall which is different
        // on windows and linux dotnet will default to the calling convention
        // for the platform that the code is running on.
        // so we need to set the calling convention to the one for which the
        // guest binary is build (windows x64 calling convention docs:
        // https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention?view=msvc-170
        // ).
        // on linux however, this isn't possible (see this document for more
        // details: https://docs.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.callingconvention?view=net-6.0)
        //
        // Alternatives:
        //
        // 1. build the binary for windows and linux and then run the correct
        // version for the platform on which we're running.
        //
        // 2. alter the calling convention of the guest binary and then tell
        // dotnet to use that calling convention. the only option for this
        // seems to be vectorcall
        // (https://docs.microsoft.com/en-us/cpp/cpp/vectorcall?view=msvc-170).
        // cdecl and stdcall are not possible using CL on x64 platform.
        // vectorcall is not supported by dotnet
        // (see https://github.com/dotnet/runtime/issues/8300)
        //
        // 3. write our own code to correct the calling convention
        //
        // 4. write epilogue/prolog code in the guest binary.
        //
        // also see https://www.agner.org/optimize/calling_conventions.pdf
        // and https://eli.thegreenplace.net/2011/09/06/stack-frame-layout-on-x86-64/

        // the following lines are here to ensure clippy/rustc doesn't
        // complain about the following parameters:
        //
        // - u_sbox being marked mut and unused
        // - outb_hdl being unused
        let _ = outb_hdl;
        log_then_return!("in-process execution is not supported on linux");
    }
    #[cfg(target_os = "windows")]
    {
        // To be able to call outb from the guest we need to provide both the
        // address of the function and a pointer to OutBHandlerWrapper.
        //
        // The guest can then call the call_outb function, passing the pointer
        // to OutBHandlerWrapper as the first argument

        // Here, we leak the outb handler, so we can write its stable address to
        // memory, and know that it won't be dropped before it's actually
        // called.
        //
        // This leaked memory is eventually dropped in the drop implementation
        // of SingleUseSandbox or MultiUseSandbox
        let leaked_outb = LeakedOutBWrapper::new(_hshm.as_mut(), outb_hdl.clone())?;
        let peb_address = {
            let base_addr = u64::try_from(_gshm.shared_mem.base_addr())?;
            _gshm.get_peb_address(base_addr, true)
        }?;
        let page_size = u32::try_from(get_os_page_size())?;
        let seed = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            rng.gen::<u64>()
        };
        unsafe { call_entry_point(&_gshm, RawPtr::from(peb_address), seed, page_size) }?;
        Ok(leaked_outb)
    }
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
fn hv_init(
    hshm: &MemMgrWrapper<HostSharedMemory>,
    gshm: SandboxMemoryManager<GuestSharedMemory>,
    host_funcs: Arc<Mutex<HostFuncsWrapper>>,
    max_init_time: Duration,
    max_exec_time: Duration,
    max_wait_for_cancellation: Duration,
) -> Result<HypervisorHandler> {
    let outb_hdl = outb_handler_wrapper(hshm.clone(), host_funcs);
    let mem_access_hdl = mem_access_handler_wrapper(hshm.clone());

    let seed = {
        let mut rng = rand::thread_rng();
        rng.gen::<u64>()
    };
    let peb_addr = {
        let peb_u64 = u64::try_from(gshm.layout.peb_address)?;
        RawPtr::from(peb_u64)
    };

    let page_size = u32::try_from(get_os_page_size())?;
    let hv_handler_config = HvHandlerConfig {
        outb_handler: outb_hdl,
        mem_access_handler: mem_access_hdl,
        seed,
        page_size,
        peb_addr,
        dispatch_function_addr: Arc::new(Mutex::new(None)),
        max_init_time,
        max_exec_time,
        max_wait_for_cancellation,
    };
    // Note: `dispatch_function_addr` is set by the Hyperlight guest library, and so it isn't in
    // shared memory at this point in time. We will set it after the execution of `hv_init`.

    let mut hv_handler = HypervisorHandler::new(hv_handler_config);

    hv_handler.start_hypervisor_handler(gshm)?;

    hv_handler
        .execute_hypervisor_handler_action(HypervisorHandlerAction::Initialise)
        .map_err(|exec_e| match hv_handler.kill_hypervisor_handler_thread() {
            Ok(_) => exec_e,
            Err(kill_e) => new_error!("{}", format!("{}, {}", exec_e, kill_e)),
        })?;

    Ok(hv_handler)
}

#[cfg(test)]
mod tests {
    use hyperlight_testing::{callback_guest_as_string, simple_guest_as_string};

    use super::evolve_impl_multi_use;
    use crate::sandbox::uninitialized::GuestBinary;
    use crate::UninitializedSandbox;

    #[test]
    fn test_evolve() {
        let guest_bin_paths = vec![
            simple_guest_as_string().unwrap(),
            callback_guest_as_string().unwrap(),
        ];
        for guest_bin_path in guest_bin_paths {
            let u_sbox = UninitializedSandbox::new(
                GuestBinary::FilePath(guest_bin_path.clone()),
                None,
                None,
                None,
            )
            .unwrap();
            evolve_impl_multi_use(u_sbox).unwrap();
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_evolve_in_proc() {
        use crate::SandboxRunOptions;

        let guest_bin_paths = vec![
            simple_guest_as_string().unwrap(),
            callback_guest_as_string().unwrap(),
        ];
        for guest_bin_path in guest_bin_paths {
            let u_sbox: UninitializedSandbox = UninitializedSandbox::new(
                GuestBinary::FilePath(guest_bin_path.clone()),
                None,
                Some(SandboxRunOptions::RunInHypervisor),
                None,
            )
            .unwrap();
            let err = format!("error evolving sandbox with guest binary {guest_bin_path}");
            let err_str = err.as_str();
            evolve_impl_multi_use(u_sbox).expect(err_str);
        }
    }
}
