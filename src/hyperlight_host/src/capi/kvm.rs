use super::context::{Context, ReadResult, WriteResult};
use super::handle::Handle;
use super::hdl::Hdl;
use crate::hypervisor::kvm;
use crate::hypervisor::kvm_mem::{map_vm_memory_region_raw, unmap_vm_memory_region_raw};
use crate::hypervisor::kvm_regs::{Regs, SRegs};
use kvm_bindings::kvm_userspace_memory_region;
use kvm_ioctls::{Kvm, VcpuFd, VmFd};
use std::os::raw::c_void;

fn get_kvm(ctx: &Context, handle: Handle) -> ReadResult<Kvm> {
    Context::get(handle, &ctx.kvms, |b| matches!(b, Hdl::Kvm(_)))
}

fn get_vmfd(ctx: &Context, handle: Handle) -> ReadResult<VmFd> {
    Context::get(handle, &ctx.kvm_vmfds, |b| matches!(b, Hdl::KvmVmFd(_)))
}

fn get_vcpufd(ctx: &Context, handle: Handle) -> ReadResult<VcpuFd> {
    Context::get(handle, &ctx.kvm_vcpufds, |b| matches!(b, Hdl::KvmVcpuFd(_)))
}

fn get_user_mem_region_mut(
    ctx: &Context,
    handle: Handle,
) -> WriteResult<kvm_userspace_memory_region> {
    Context::get_mut(handle, &ctx.kvm_user_mem_regions, |b| {
        matches!(b, Hdl::KvmUserMemRegion(_))
    })
}

fn get_kvm_run_message(ctx: &Context, handle: Handle) -> ReadResult<kvm::KvmRunMessage> {
    Context::get(handle, &ctx.kvm_run_messages, |b| {
        matches!(b, Hdl::KvmRunMessage(_))
    })
}

/// Returns a bool indicating if hyperv is present on the machine
/// Takes an argument to indicate if the hypervisor api must be stable
/// If the hypervisor api is not stable, the function will return false even if the hypervisor is present
///
/// # Examples
///
/// ```
/// use hyperlight_host::capi::hyperv_linux::is_hyperv_linux_present;
///
/// assert_eq!(is_hyperv_linux_present(require_stable_api), true );
/// ```
#[no_mangle]
pub extern "C" fn kvm_is_present() -> bool {
    // At this point we dont have any way to report the error if one occurs.
    kvm::is_present().map(|_| true).unwrap_or(false)
}

/// Open a Handle to KVM. Returns a handle to a KVM or a `Handle` to an error
/// if there was an issue.
///
/// The caller is responsible for closing the handle by passing it
/// to `handle_free` exactly once after they're done using it.
/// Doing so will not only free the memory that was allocated by
/// this function, it will also free all internal resources connected to
/// the associated VM, such as the underlying file descriptor.
///
/// No explicit close function (i.e. `kvm_close`) is needed or provided.
///
/// # Safety
///
/// You must free this handle by calling `handle_free` exactly once
/// after you're done using it.
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_open(ctx: *mut Context) -> Handle {
    match kvm::open() {
        Ok(k) => Context::register(k, &(*ctx).kvms, Hdl::Kvm),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get size of memory map required to pass to kvm_run
///
/// # Safety
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `open_mshv`
#[no_mangle]
pub unsafe extern "C" fn kvm_get_mmap_size(ctx: *const Context, kvm_fd_hdl: Handle) -> usize {
    let kvm = match get_kvm(&*ctx, kvm_fd_hdl) {
        Ok(k) => k,
        Err(_) => return 0,
    };
    kvm::get_mmap_size(&*kvm).unwrap_or(0)
}

/// Create a VM and return a Handle to it. Returns a handle to a VM or a `Handle` to an error
/// if there was an issue.
///
/// # Safety
///
/// You must free this handle by calling `handle_free` exactly once
/// after you're done using it.
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `open_mshv`
///
/// 2. `Handle` to a `Mshv` that has been:
/// - Created with `open_mshv`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_create_vm(ctx: *mut Context, kvm_handle: Handle) -> Handle {
    let kvm = match get_kvm(&*ctx, kvm_handle) {
        Ok(kvm) => kvm,
        Err(e) => return (*ctx).register_err(e),
    };
    match kvm::create_vm(&*kvm) {
        Ok(vm_fd) => Context::register(vm_fd, &(*ctx).kvm_vmfds, Hdl::KvmVmFd),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Create a KVM vCPU and return a Handle to it.
/// Returns a handle to a vCPU or a `Handle` to an error if there was an
/// issue.
///
/// # Safety
///
/// You must free this handle by calling `handle_free` exactly once
/// after you're done using it.
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `open_mshv`
/// - Used to call `create_vm`
///
/// 2. `Handle` to a `VmFd` that has been:
/// - Created with `create_vm`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_create_vcpu(ctx: *mut Context, vmfd_hdl: Handle) -> Handle {
    let vmfd = match get_vmfd(&*ctx, vmfd_hdl) {
        Ok(vmfd) => vmfd,
        Err(e) => return (*ctx).register_err(e),
    };
    match kvm::create_vcpu(&vmfd) {
        Ok(res) => Context::register(res, &(*ctx).kvm_vcpufds, Hdl::KvmVcpuFd),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Map a memory region in the host to the VM and return a Handle to it. Returns a handle to a mshv_user_mem_region or a `Handle` to an error
/// if there was an issue.
///
/// # Safety
///
/// You must destory this handle by calling `kvm_unmap_vm_memory_region` exactly once
/// after you're done using it.
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `open_mshv`
/// - Used to call `create_vm`
///
/// 2. `Handle` to a `VmFd` that has been:
/// - Created with `create_vm`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
///
/// 3. The guest Page Frame Number (this can be calculated by right bit shifting the guest base address by 12 e.g. BaseAddress >> 12)
///
/// 4. The load address of the memory region being mapped (this is the address of the memory in the host process)
///
/// 5. The size of the memory region being mapped (this is the size of the memory allocated at load_address)
#[no_mangle]
pub unsafe extern "C" fn kvm_map_vm_memory_region(
    ctx: *mut Context,
    vmfd_hdl: Handle,
    guest_phys_addr: u64,
    userspace_addr: *const c_void,
    mem_size: u64,
) -> Handle {
    let vmfd = match get_vmfd(&*ctx, vmfd_hdl) {
        Ok(r) => r,
        Err(e) => return (*ctx).register_err(e),
    };
    match map_vm_memory_region_raw(&vmfd, guest_phys_addr, userspace_addr, mem_size) {
        Ok(mem_region) => Context::register(
            mem_region,
            &(*ctx).kvm_user_mem_regions,
            Hdl::KvmUserMemRegion,
        ),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Unmap a memory region in the host to the VM and return a Handle to it. Returns an empty handle or a `Handle` to an error
/// if there was an issue.
///
/// # Safety
///
/// If the retruned handle is a Handle to an error then it should be freed by calling `handle_free` .The empty handle does not need to be freed but calling `handle_free` is will not cause an error.
/// The `mshv_user_mem_regions_handle` handle passed to this function should be freed after the call using `free_handle`.
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `open_mshv`
/// - Used to call `create_vm`
///
/// 2. `Handle` to a `VmFd` that has been:
/// - Created with `create_vm`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
///
/// 3. `Handle` to a `mshv_user_mem_region` that has been:
/// - Created with `map_vm_memory_region`
/// - Not unmapped and freed by calling this function
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_unmap_vm_memory_region(
    ctx: *mut Context,
    vmfd_hdl: Handle,
    user_mem_region_hdl: Handle,
) -> Handle {
    let vmfd = match get_vmfd(&*ctx, vmfd_hdl) {
        Ok(r) => r,
        Err(e) => return (*ctx).register_err(e),
    };
    let mut mem_region = match get_user_mem_region_mut(&*ctx, user_mem_region_hdl) {
        Ok(r) => r,
        Err(e) => return (*ctx).register_err(e),
    };
    match unmap_vm_memory_region_raw(&*vmfd, &mut *mem_region) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get registers from the vCPU. Returns a `Handle` holding a reference
/// to registers or a `Handle referencing an error if there was an issue.
/// Fetch the registers from a successful `Handle` with
/// `kvm_get_registers_from_handle`.
///
/// # Safety
///
/// If the handle is a Handle to an error then it should be freed by
/// calling `handle_free`.
/// The empty handle does not need to be freed but calling `handle_free`
/// will not cause an error.
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `kvm_open`
/// - Used to call `kvm_create_vm`
/// - Used to call `kvm_create_vcpu`
///
/// 2. `Handle` to a `VcpuFd` that has been:
/// - Created with `create_vcpu`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
///
/// 3. A valid `kvm_regs` instance
#[no_mangle]
pub unsafe extern "C" fn kvm_get_registers(ctx: *mut Context, vcpufd_hdl: Handle) -> Handle {
    let vcpufd = match get_vcpufd(&*ctx, vcpufd_hdl) {
        Ok(r) => r,
        Err(e) => return (*ctx).register_err(e),
    };
    match kvm::get_registers(&*vcpufd) {
        Ok(regs) => Context::register(regs, &(*ctx).kvm_regs, Hdl::KvmRegisters),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get registers from a handle created by `kvm_get_registers`.
///
/// Returns either a pointer to the registers or `NULL`.
///
/// # Safety
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `kvm_open`
/// - Used to call `kvm_create_vm`
/// - Used to call `kvm_create_vcpu`
///
/// 2. `Handle` to a registers struct that has been created by
/// a call to `kvm_get_registers`
///
/// If this function returns a non-`NULL` pointer, the caller is responsible
/// for calling `free` on that pointer when they're done with the memory.
#[no_mangle]
pub unsafe extern "C" fn kvm_get_registers_from_handle(
    ctx: *const Context,
    regs_hdl: Handle,
) -> *mut Regs {
    match Context::get(regs_hdl, &((*ctx).kvm_regs), |h| {
        matches!(h, Hdl::KvmRegisters(_))
    }) {
        Ok(r) => Box::into_raw(Box::new(*r)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get segment registers from the vCPU. Returns a `Handle` holding a reference
/// to registers or a `Handle referencing an error if there was an issue.
/// Fetch the registers from a successful `Handle` with
/// `kvm_get_registers_from_handle`.
///
/// # Safety
///
/// If the handle is a Handle to an error then it should be freed by
/// calling `handle_free`.
/// The empty handle does not need to be freed but calling `handle_free`
/// will not cause an error.
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `kvm_open`
/// - Used to call `kvm_create_vm`
/// - Used to call `kvm_create_vcpu`
///
/// 2. `Handle` to a `VcpuFd` that has been:
/// - Created with `create_vcpu`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
///
/// 3. A valid `kvm_regs` instance
#[no_mangle]
pub unsafe extern "C" fn kvm_get_sregisters(ctx: *mut Context, vcpufd_hdl: Handle) -> Handle {
    let vcpufd = match get_vcpufd(&*ctx, vcpufd_hdl) {
        Ok(r) => r,
        Err(e) => return (*ctx).register_err(e),
    };
    match kvm::get_sregisters(&*vcpufd) {
        Ok(regs) => Context::register(regs, &(*ctx).kvm_sregs, Hdl::KvmSRegisters),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Get registers from a handle created by `kvm_get_registers`.
///
/// Returns either a pointer to the registers or `NULL`.
///
/// # Safety
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `kvm_open`
/// - Used to call `kvm_create_vm`
/// - Used to call `kvm_create_vcpu`
///
/// 2. `Handle` to a registers struct that has been created by
/// a call to `kvm_get_registers`
///
/// If this function returns a non-`NULL` pointer, the caller is responsible
/// for calling `free` on that pointer when they're done with the memory.
#[no_mangle]
pub unsafe extern "C" fn kvm_get_sregisters_from_handle(
    ctx: *const Context,
    sregs_hdl: Handle,
) -> *mut SRegs {
    match Context::get(sregs_hdl, &((*ctx).kvm_sregs), |h| {
        matches!(h, Hdl::KvmSRegisters(_))
    }) {
        Ok(r) => Box::into_raw(Box::new(*r)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Set Registers in the vCPU. Returns an empty handle or a `Handle` to
/// an error if there was an issue.
///
/// # Safety
///
/// If the handle is a Handle to an error then it should be freed by
/// calling `handle_free`.
/// The empty handle does not need to be freed but calling `handle_free`
/// will not cause an error.
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `kvm_open`
/// - Used to call `kvm_create_vm`
/// - Used to call `kvm_create_vcpu`
///
/// 2. `Handle` to a `VcpuFd` that has been:
/// - Created with `create_vcpu`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
///
/// 3. A valid `kvm_regs` instance
#[no_mangle]
pub unsafe extern "C" fn kvm_set_registers(
    ctx: *mut Context,
    vcpufd_hdl: Handle,
    // TODO: consider passing this by reference or creating a new
    // Handle type for registers and passing a handle here.
    regs: Regs,
) -> Handle {
    let vcpu_fd = match get_vcpufd(&*ctx, vcpufd_hdl) {
        Ok(r) => r,
        Err(e) => return (*ctx).register_err(e),
    };
    // TODO: create a RegisterArray similar to ByteArray here?
    match kvm::set_registers(&*vcpu_fd, &regs) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Set segment registers `sregs` on the vcpu stored in `ctx` referenced
/// by `vcpufd_hdl`.
///
/// # Safety
///
/// If the handle is a Handle to an error then it should be freed by
/// calling `handle_free`.
/// The empty handle does not need to be freed but calling `handle_free`
/// will not cause an error.
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `kvm_open`
/// - Used to call `kvm_create_vm`
/// - Used to call `kvm_create_vcpu`
///
/// 2. `Handle` to a `VcpuFd` that has been:
/// - Created with `create_vcpu`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
///
/// 3. A valid `kvm_regs` instance
#[no_mangle]
pub unsafe extern "C" fn kvm_set_sregisters(
    ctx: *mut Context,
    vcpufd_hdl: Handle,
    sregs: SRegs,
) -> Handle {
    let vcpu_fd = match get_vcpufd(&*ctx, vcpufd_hdl) {
        Ok(r) => r,
        Err(e) => return (*ctx).register_err(e),
    };
    match kvm::set_sregisters(&*vcpu_fd, &sregs) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Runs a vCPU. Returns an handle to an `kvm_run_message` or a
/// `Handle` to an error if there was an issue.
///
/// # Safety
///
/// The returned handle is a handle to an `kvm_run_message`.
/// The  corresponding `kvm_run_message`
/// should be retrieved using `kvm_get_run_result_from_handle`.
/// The handle should be freed by calling `handle_free` once the message
/// has been retrieved.
///
/// You must call this function with
///
/// 1. A `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `kvm_open`
/// - Used to call `kvm_create_vm`
/// - Used to call `kvm_create_vcpu`
/// - Used to call `kvm_set_registers`
///
/// 2. `Handle` to a `VcpuFd` that has been:
/// - Created with `kvm_create_vcpu`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_run_vcpu(ctx: *mut Context, vcpufd_hdl: Handle) -> Handle {
    let vcpu_fd = match get_vcpufd(&*ctx, vcpufd_hdl) {
        Ok(r) => r,
        Err(e) => return (*ctx).register_err(e),
    };
    match kvm::run_vcpu(&*vcpu_fd) {
        Ok(run_result) => {
            Context::register(run_result, &(*ctx).kvm_run_messages, Hdl::KvmRunMessage)
        }
        Err(e) => (*ctx).register_err(e),
    }
}

/// Gets the `kvm_run_message` associated with the given handle.
///
/// # Safety
///
/// Both the returned `kvm_run_message` and the given `handle` should
/// be freed by the called when they're no longer in use. The former
/// should be freed with `free` and the latter with `handle_free`.
///
/// You must call this function with
///
/// 1. `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free
/// - Not modified, except by calling functions in the Hyperlight C API
/// - Used to call `kvm_open`
/// - Used to call `kvm_create_vm`
/// - Used to call `kvm_create_vcpu`
/// - Used to call `kvm_set_registers`
/// - Used to call `kvm_run_vcpu`
///
/// 2. `Handle` to a `kvm_run_message` that has been:
/// - Created with `kvm_run_vcpu`
/// - Not yet used to call this function
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn kvm_get_run_result_from_handle(
    ctx: *mut Context,
    handle: Handle,
) -> *const kvm::KvmRunMessage {
    let result = match get_kvm_run_message(&*ctx, handle) {
        Ok(res) => res,
        Err(_) => return std::ptr::null(),
    };
    // TODO: Investigate why calling (*ctx).remove(hdl, |_| true) hangs here.
    // This would be a better way to do things...
    Box::into_raw(Box::new(*result))
}

/// Frees a `kvm_run_message` previously returned by
/// `kvm_get_run_result_from_handle`.
///
/// see https://doc.rust-lang.org/std/boxed/index.html#memory-layout
/// for information on how the mechanics of this function work.
///
/// # Safety
///
/// You must call this function with
///
///
/// 1. A Pointer to a previously returned  `kvm_run_message` from
/// `kvm_get_run_result_from_handle`.
/// - Created with `kvm_get_run_result_from_handle`
/// - Not yet used to call this function
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub extern "C" fn kvm_free_run_result(_: Option<Box<kvm::KvmRunMessage>>) {}
