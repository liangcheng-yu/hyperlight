use std::sync::{Arc, Mutex};

use hyperlight_host::hypervisor::hyperv_linux::{is_hypervisor_present, HypervLinuxDriver};
use hyperlight_host::hypervisor::{Hypervisor, VirtualCPU};
use hyperlight_host::mem::ptr::{GuestPtr, RawPtr};
use hyperlight_host::Result;

use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::mem_access_handler::{get_mem_access_handler_func, MemAccessHandlerWrapper};
use super::outb_handler::get_outb_handler_func;
use crate::c_func::CFunc;
use crate::mem_mgr::get_mem_mgr;
use crate::outb_handler::OutBHandlerWrapper;
use crate::validate_context;

fn get_driver_mut(ctx: &mut Context, hdl: Handle) -> Result<&mut HypervLinuxDriver> {
    Context::get_mut(hdl, &mut ctx.hyperv_linux_drivers, |h| {
        matches!(h, Hdl::HypervLinuxDriver(_))
    })
}

#[allow(dead_code)] // most certainly will be useful in the future
fn get_driver(ctx: &Context, hdl: Handle) -> Result<&HypervLinuxDriver> {
    Context::get(hdl, &ctx.hyperv_linux_drivers, |h| {
        matches!(h, Hdl::HypervLinuxDriver(_))
    })
}

/// Returns a bool indicating if hyperv is present on the machine.
///
/// # Examples
///
/// ```
/// use hyperlight_host::capi::hyperv_linux::is_hyperv_linux_present;
///
/// assert_eq!(is_hyperv_linux_present(), true );
/// ```
#[no_mangle]
pub extern "C" fn is_hyperv_linux_present() -> bool {
    // At this point we don't have any way to report the error if one occurs.
    is_hypervisor_present()
}

/// Creates a new HyperV-Linux driver with the given parameters and
/// "advanced" registers, suitable for a guest program that access
/// memory.
///
/// If the driver was created successfully, returns a `Handle` referencing the
/// new driver. Otherwise, returns a new `Handle` that references a descriptive
/// error.
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn hyperv_linux_create_driver(
    ctx: *mut Context,
    mgr_hdl: Handle,
    entrypoint: u64,
    rsp: u64,
    pml4: u64,
) -> Handle {
    CFunc::new("hyperv_linux_create_driver", ctx)
        .and_then_mut(|ctx, _| {
            let entrypoint_ptr = GuestPtr::try_from(RawPtr::from(entrypoint))?;
            let rsp_ptr = GuestPtr::try_from(RawPtr::from(rsp))?;
            let pml4_ptr = GuestPtr::try_from(RawPtr::from(pml4))?;

            let mgr = get_mem_mgr(ctx, mgr_hdl)?;
            let driver = HypervLinuxDriver::new(
                mgr.layout.get_memory_regions(&mgr.shared_mem),
                entrypoint_ptr,
                rsp_ptr,
                pml4_ptr,
            )?;
            Ok(Context::register(
                driver,
                &mut ctx.hyperv_linux_drivers,
                Hdl::HypervLinuxDriver,
            ))
        })
        .ok_or_err_hdl()
}

pub(crate) fn get_handler_funcs(
    ctx: &Context,
    outb_func_hdl: Handle,
    mem_access_func_hdl: Handle,
) -> Result<(OutBHandlerWrapper, MemAccessHandlerWrapper)> {
    let outb_func = get_outb_handler_func(ctx, outb_func_hdl).map(|f| (*f).clone())?;
    let mem_access_func =
        get_mem_access_handler_func(ctx, mem_access_func_hdl).map(|f| (*f).clone())?;
    Ok((outb_func, mem_access_func))
}

/// Initialise the vCPU, call the equivalent of `VirtualCPU::run`,
/// and return the result.
///
/// Return an empty `Handle` on success, or a `Handle` that references a
/// descriptive error on failure.
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn hyperv_linux_initialise(
    ctx: *mut Context,
    driver_hdl: Handle,
    outb_func_hdl: Handle,
    mem_access_func_hdl: Handle,
    peb_addr: u64,
    seed: u64,
    page_size: u32,
) -> Handle {
    validate_context!(ctx);
    let driver = match get_driver_mut(&mut *ctx, driver_hdl) {
        Ok(d) => d,
        Err(e) => return (*ctx).register_err(e),
    };
    let (outb_func, mem_access_func) =
        match get_handler_funcs(&*ctx, outb_func_hdl, mem_access_func_hdl) {
            Ok(tup) => tup,
            Err(e) => return (*ctx).register_err(e),
        };
    let init_res = (*driver).initialise(
        peb_addr.into(),
        seed,
        page_size,
        Arc::new(Mutex::new(outb_func)),
        Arc::new(Mutex::new(mem_access_func)),
    );
    match init_res {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Execute the virtual CPU stored inside the HyperV Linux driver referenced
/// by `driver_hdl` until a HLT instruction is reached. You likely should
/// call `hyperv_linux_initialise` instead of this function.
///
/// Return an empty `Handle` on success, or a `Handle` that references a
/// descriptive error on failure.
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn hyperv_linux_run_vcpu(
    ctx: *mut Context,
    driver_hdl: Handle,
    outb_func_hdl: Handle,
    mem_access_func_hdl: Handle,
) -> Handle {
    validate_context!(ctx);
    let driver = match get_driver_mut(&mut *ctx, driver_hdl) {
        Ok(d) => d,
        Err(e) => return (*ctx).register_err(e),
    };
    let (outb_func, mem_access_func) =
        match get_handler_funcs(&*ctx, outb_func_hdl, mem_access_func_hdl) {
            Ok(tup) => tup,
            Err(e) => return (*ctx).register_err(e),
        };
    match VirtualCPU::run(
        (*driver).as_mut_hypervisor(),
        Arc::new(Mutex::new(outb_func)),
        Arc::new(Mutex::new(mem_access_func)),
    ) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Dispatch a call from the host to the guest, using the function
/// referenced by `dispatch_func_addr`
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
///
#[no_mangle]
pub unsafe extern "C" fn hyperv_linux_dispatch_call_from_host(
    ctx: *mut Context,
    driver_hdl: Handle,
    outb_func_hdl: Handle,
    mem_access_func_hdl: Handle,
    dispatch_func_addr: u64,
) -> Handle {
    validate_context!(ctx);
    let driver = match get_driver_mut(&mut *ctx, driver_hdl) {
        Ok(d) => d,
        Err(e) => return (*ctx).register_err(e),
    };
    let (outb_func, mem_access_func) =
        match get_handler_funcs(&*ctx, outb_func_hdl, mem_access_func_hdl) {
            Ok(tup) => tup,
            Err(e) => return (*ctx).register_err(e),
        };
    match (*driver).dispatch_call_from_host(
        dispatch_func_addr.into(),
        Arc::new(Mutex::new(outb_func)),
        Arc::new(Mutex::new(mem_access_func)),
    ) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}
