use super::{leaked_outb::LeakedOutBWrapper, WrapperGetter};
use crate::hypervisor::hypervisor_handler::InitArgs;
use crate::hypervisor::hypervisor_handler::VCPUAction;
use crate::hypervisor::hypervisor_handler::{start_hypervisor_handler, HandlerMsg};
#[cfg(target_os = "linux")]
use crate::log_then_return;
use crate::HyperlightError::HypervisorHandlerCommunicationFailure;
use crate::{
    func::exports::get_os_page_size, hypervisor::handlers::MemAccessHandlerWrapper,
    mem::ptr::RawPtr, new_error, MultiUseSandbox,
};
use crate::{
    hypervisor::handlers::OutBHandlerWrapper, sandbox_state::sandbox::Sandbox, SingleUseSandbox,
    UninitializedSandbox,
};
use crate::{HyperlightError, Result};
use rand::Rng;
use std::thread::JoinHandle;
use tracing::{instrument, Span};

pub(super) type CBFunc<'a> = Box<dyn FnOnce(&mut UninitializedSandbox) -> Result<()> + 'a>;

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
    mut u_sbox: UninitializedSandbox,
    cb_opt: Option<CBFunc<'a>>,
    transform: TransformFunc,
) -> Result<ResSandbox>
where
    TransformFunc: Fn(
        UninitializedSandbox,
        Option<JoinHandle<Result<()>>>,
        Option<LeakedOutBWrapper<'a>>,
    ) -> Result<ResSandbox>,
{
    let outb_wrapper = {
        let hv = u_sbox.get_hv();
        hv.outb_hdl.clone()
    };
    let run_from_proc_mem = u_sbox.run_from_process_memory;
    let mut join_handle = None;

    let leaked_outb = if run_from_proc_mem {
        let leaked_outb = evolve_in_proc(&mut u_sbox, outb_wrapper)?;
        Some(leaked_outb)
    } else {
        let orig_rsp = u_sbox.get_hv().get_hypervisor_lock()?.orig_rsp()?;
        join_handle = Some(hv_init(
            &u_sbox,
            u_sbox.hv.outb_hdl.clone(),
            u_sbox.hv.mem_access_hdl.clone(),
        )?);

        {
            let mgr = u_sbox.mgr.as_ref();
            assert_ne!(mgr.get_pointer_to_dispatch_function()?, 0);
        }

        // TODO: this if check makes no sense , first getting a hypervisor when running in process is not going to work
        // secondly, this branch of the if statement on line 45 means this could never be true
        // so we should probably remove this if statement and just call the function
        if u_sbox.run_from_process_memory {
            u_sbox
                .get_hv_mut()
                .get_hypervisor_lock()?
                .reset_rsp(orig_rsp)?;
        }
        None
    };
    if let Some(cb) = cb_opt {
        cb(&mut u_sbox)?;
    }

    transform(u_sbox, join_handle, leaked_outb)
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
pub(super) fn evolve_impl_multi_use(
    u_sbox: UninitializedSandbox,
    cb_opt: Option<CBFunc>,
) -> Result<MultiUseSandbox> {
    evolve_impl(u_sbox, cb_opt, |mut u, join_handle, leaked_outb| {
        {
            u.get_mgr_wrapper_mut().as_mut().push_state()?;
        }
        Ok(MultiUseSandbox::from_uninit(u, join_handle, leaked_outb))
    })
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
pub(super) fn evolve_impl_single_use(
    u_sbox: UninitializedSandbox,
    cb_opt: Option<CBFunc>,
) -> Result<SingleUseSandbox> {
    evolve_impl(u_sbox, cb_opt, |u, join_handle, leaked_outb| {
        // Its intentional not to snapshot state here. This is because
        // single use sandboxes are not reusable and so there is no need
        // to snapshot state as they cannot be devolved back to an uninitialized sandbox.
        Ok(SingleUseSandbox::from_uninit(u, join_handle, leaked_outb))
    })
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
fn evolve_in_proc<'a>(
    u_sbox: &mut UninitializedSandbox,
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
        let _ = u_sbox.get_mgr_wrapper();
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
        let mgr = u_sbox.get_mem_mgr_mut();
        let leaked_outb = LeakedOutBWrapper::new(mgr, outb_hdl.clone())?;
        let peb_address = {
            let base_addr = u64::try_from(mgr.shared_mem.base_addr())?;
            mgr.get_peb_address(base_addr)
        }?;
        let page_size = u32::try_from(get_os_page_size())?;
        let seed = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            rng.gen::<u64>()
        };
        unsafe { u_sbox.call_entry_point(RawPtr::from(peb_address), seed, page_size) }?;
        Ok(leaked_outb)
    }
}

#[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
fn hv_init(
    u_sbox: &UninitializedSandbox,
    outb_hdl: OutBHandlerWrapper,
    mem_access_hdl: MemAccessHandlerWrapper,
) -> Result<JoinHandle<Result<()>>> {
    let mem_mgr = u_sbox.get_mgr_wrapper().unwrap_mgr();
    let seed = {
        let mut rng = rand::thread_rng();
        rng.gen::<u64>()
    };
    let peb_addr = {
        let peb_u64 = u64::try_from(mem_mgr.layout.peb_address)?;
        RawPtr::from(peb_u64)
    };
    let page_size = u32::try_from(get_os_page_size())?;
    let outb_hdl = outb_hdl.clone();
    let mem_access_hdl = mem_access_hdl.clone();

    let hv_arc = u_sbox.get_hv().get_hypervisor_arc()?;
    let (to_handler_tx, to_handler_rx, from_handler_tx, from_handler_rx) = {
        let mut hv_lock = hv_arc.lock().expect("failed to lock hypervisor");
        let (from_handler_tx, to_handler_rx) = hv_lock.setup_handler_channels();

        let to_handler_tx = hv_lock.get_to_handler_tx();
        let from_handler_rx = hv_lock.get_from_handler_rx();

        (
            to_handler_tx,
            to_handler_rx,
            from_handler_tx,
            from_handler_rx,
        )
    };
    let join_handle = start_hypervisor_handler(hv_arc, (from_handler_tx, to_handler_rx))?;

    // send message to initialise hypervisor
    to_handler_tx
        .send(VCPUAction::Initialise(InitArgs::new(
            peb_addr,
            seed,
            page_size,
            outb_hdl,
            mem_access_hdl,
        )))
        .map_err(|_| HyperlightError::HypervisorHandlerCommunicationFailure())?;

    // wait for hypervisor to finish initialising
    if let HandlerMsg::Error(e) = from_handler_rx
        .recv()
        .map_err(|_| HypervisorHandlerCommunicationFailure())?
    {
        join_handle
            .join()
            .map_err(|_| new_error!("failed to join hypervisor handler thread"))??;

        return Err(e);
    }

    Ok(join_handle)
}

#[cfg(test)]
mod tests {
    use super::evolve_impl_multi_use;
    use crate::{sandbox::uninitialized::GuestBinary, UninitializedSandbox};
    use hyperlight_testing::{callback_guest_as_string, simple_guest_as_string};

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
            evolve_impl_multi_use(u_sbox, None).unwrap();
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
            evolve_impl_multi_use(u_sbox, None).expect(err_str);
        }
    }
}
