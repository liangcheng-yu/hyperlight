use super::context::Context;
use super::handle::Handle;
use super::hdl::Hdl;
use super::hyperv_linux::get_handler_funcs;
use crate::c_func::CFunc;
use anyhow::Result;
use hyperlight_host::hypervisor::{
    kvm::{self, KVMDriver},
    Hypervisor,
};
use std::sync::{Arc, Mutex};

fn get_driver_mut(ctx: &mut Context, handle: Handle) -> Result<&mut KVMDriver> {
    Context::get_mut(handle, &mut ctx.kvm_drivers, |b| {
        matches!(b, Hdl::KVMDriver(_))
    })
}

/// Returns a bool indicating if kvm is present on the machine
///
/// # Examples
///
/// ```
/// use hyperlight_host::capi::kvm::kvm_is_present;
///
/// assert_eq!(kvm::kvm_is_present(), true );
/// ```
#[no_mangle]
pub extern "C" fn is_kvm_present() -> bool {
    // At this point we dont have any way to report the error if one occurs.
    kvm::is_hypervisor_present().map(|_| true).unwrap_or(false)
}

/// Creates a new KVM driver with the given parameters
///
/// If the driver was created successfully, returns a `Handle` referencing the
/// new driver. Otherwise, returns a new `Handle` that references a descriptive
/// error.
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_create_driver(
    ctx: *mut Context,
    source_addr: u64,
    pml4_addr: u64,
    mem_size: u64,
    entrypoint: u64,
    rsp: u64,
) -> Handle {
    CFunc::new("kvm_create_driver", ctx)
        .and_then_mut(|ctx, _| {
            let driver = KVMDriver::new(source_addr, pml4_addr, mem_size, entrypoint, rsp)?;
            Ok(Context::register(
                driver,
                &mut ctx.kvm_drivers,
                Hdl::KVMDriver,
            ))
        })
        .ok_or_err_hdl()
}

/// Set the stack pointer register.
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_set_rsp(
    ctx: *mut Context,
    driver_hdl: Handle,
    rsp_val: u64,
) -> Handle {
    CFunc::new("kvm_set_rsp", ctx)
        .and_then_mut(|ctx, _| {
            let driver = get_driver_mut(ctx, driver_hdl)?;
            driver.reset_rsp(rsp_val).map(|_| Handle::new_empty())
        })
        .ok_or_err_hdl()
}

/// Initialise the vCPU, call the equivalent of `execute_until_halt`,
/// and return the result.
///
/// Return an empty `Handle` on success, or a `Handle` that references a
/// descriptive error on failure.
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_initialise(
    ctx: *mut Context,
    driver_hdl: Handle,
    outb_func_hdl: Handle,
    mem_access_func_hdl: Handle,
    peb_addr: u64,
    seed: u64,
    page_size: u32,
) -> Handle {
    CFunc::new("kvm_initialise", ctx)
        .and_then(|ctx, _| get_handler_funcs(ctx, outb_func_hdl, mem_access_func_hdl))
        .and_then_mut(|ctx, (outb_func, mem_access_func)| {
            let driver = get_driver_mut(ctx, driver_hdl)?;
            (*driver)
                .initialise(
                    peb_addr.into(),
                    seed,
                    page_size,
                    Arc::new(Mutex::new(outb_func)),
                    Arc::new(Mutex::new(mem_access_func)),
                )
                .map(|_| Handle::new_empty())
        })
        .ok_or_err_hdl()
}

/// Dispatch a call from the host to the guest, using the function
/// referenced by `dispatch_func_addr`
///
/// # Safety
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_dispatch_call_from_host(
    ctx: *mut Context,
    driver_hdl: Handle,
    outb_func_hdl: Handle,
    mem_access_func_hdl: Handle,
    dispatch_func_addr: u64,
) -> Handle {
    CFunc::new("kvm_dispatch_call_from_host", ctx)
        .and_then(|ctx, _| get_handler_funcs(ctx, outb_func_hdl, mem_access_func_hdl))
        .and_then_mut(|ctx, (outb_func, mem_access_func)| {
            let driver = get_driver_mut(ctx, driver_hdl)?;
            (*driver)
                .dispatch_call_from_host(
                    dispatch_func_addr.into(),
                    Arc::new(Mutex::new(outb_func)),
                    Arc::new(Mutex::new(mem_access_func)),
                )
                .map(|_| Handle::new_empty())
        })
        .ok_or_err_hdl()
}
