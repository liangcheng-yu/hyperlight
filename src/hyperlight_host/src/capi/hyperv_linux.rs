#![deny(missing_docs)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_macros)]
#![allow(unused_imports)]
use super::context::{Context, ReadResult};
use super::handle::{handle_free, Handle};
use super::hdl::Hdl;
use anyhow::{bail, Error, Result};
use mshv_bindings::{
    __u32, __u64, hv_message, hv_message_type, hv_register_assoc, hv_register_value, hv_u128,
    mshv_user_mem_region,
};
use mshv_ioctls::{Mshv, VcpuFd, VmFd};
use std::os::raw::{c_uint, c_ulonglong};
use std::{panic::catch_unwind, panic::RefUnwindSafe, slice};

mod impls {
    use crate::capi::context::Context;
    use crate::capi::handle::Handle;
    use crate::capi::hdl::Hdl;
    use anyhow::{Error, Result};
    use mshv_bindings::*;
    use mshv_ioctls::{Mshv, VcpuFd, VmFd};
    use std::{panic::catch_unwind, panic::RefUnwindSafe, ptr, slice};

    pub const HV_MAP_GPA_READABLE: u32 = 1;
    pub const HV_MAP_GPA_WRITABLE: u32 = 2;
    pub const HV_MAP_GPA_EXECUTABLE: u32 = 12;

    pub fn is_hypervisor_present(require_stable_api: bool) -> Result<bool> {
        let mshv = Mshv::new()?;
        match mshv.check_stable() {
            Ok(stable) => match stable {
                true => Ok(true),
                false => match require_stable_api {
                    true => Ok(false),
                    false => Ok(true),
                },
            },
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    pub fn open_mshv(require_stable_api: bool) -> Result<Mshv> {
        match is_hypervisor_present(require_stable_api) {
            Ok(true) => match Mshv::new() {
                Ok(mshv) => Ok(mshv),
                Err(e) => Err(anyhow::Error::from(e)),
            },
            Ok(false) => Err(anyhow::anyhow!(
                "Hypervisor not present (stable api was {:?})",
                require_stable_api
            )),
            Err(e) => Err(e),
        }
    }

    pub fn create_vm(mshv: &Mshv) -> Result<VmFd> {
        let pr = Default::default();
        match mshv.create_vm_with_config(&pr) {
            Ok(vmfd) => Ok(vmfd),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    pub fn create_vcpu(vmfd: &VmFd) -> Result<VcpuFd> {
        match vmfd.create_vcpu(0) {
            Ok(vcpuFd) => Ok(vcpuFd),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    pub fn map_vm_memory_region(
        vmfd: &VmFd,
        guest_pfn: u64,
        load_address: u64,
        size: u64,
    ) -> Result<mshv_user_mem_region> {
        let user_memory_region = mshv_user_mem_region {
            flags: HV_MAP_GPA_READABLE | HV_MAP_GPA_WRITABLE | HV_MAP_GPA_EXECUTABLE,
            guest_pfn,
            size,
            userspace_addr: load_address as u64,
        };

        match vmfd.map_user_memory(user_memory_region) {
            Ok(_) => Ok(user_memory_region),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    pub fn unmap_vm_memory_region(
        vmfd: &VmFd,
        user_memory_region: &mshv_user_mem_region,
    ) -> Result<()> {
        match vmfd.unmap_user_memory(*user_memory_region) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    pub fn set_registers(vcpuFd: &VcpuFd, registers: &[hv_register_assoc]) -> Result<()> {
        match vcpuFd.set_reg(registers) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    pub fn run_vcpu(vcpuFd: &VcpuFd) -> Result<hv_message> {
        let hv_message: hv_message = Default::default();
        match vcpuFd.run(hv_message) {
            Ok(result) => Ok(result),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }
}

/// CR0 Register
pub const HV_X64_REGISTER_CR0: u32 = 262144;
/// CR3 Register
pub const HV_X64_REGISTER_CR3: u32 = 262146;
/// CR4 Register
pub const HV_X64_REGISTER_CR4: u32 = 262147;
/// EFER Register
pub const HV_X64_REGISTER_EFER: u32 = 524289;
/// RAX Register
pub const HV_X64_REGISTER_RAX: u32 = 131072;
/// RBX Register
pub const HV_X64_REGISTER_RBX: u32 = 131075;
/// RIP Register
pub const HV_X64_REGISTER_RIP: u32 = 131088;
/// RFLAGS Register
pub const HV_X64_REGISTER_RFLAGS: u32 = 131089;
/// CS Register
pub const HV_X64_REGISTER_CS: u32 = 393217;
/// RSP Register
pub const HV_X64_REGISTER_RSP: u32 = 131076;
/// RCX Register
pub const HV_X64_REGISTER_RCX: u32 = 131073;

/// Returns a bool indicating if hyperv is present on the machine
/// Takes an argument to indicate if the hypervisor api must be stable
/// If the hypervisor api is not stable, the function will return false even if the hypervisor is present

#[no_mangle]
pub extern "C" fn is_hyperv_linux_present(require_stable_api: bool) -> bool {
    // At this point we dont have any way to report the error if one occurs.
    impls::is_hypervisor_present(require_stable_api).unwrap_or(false)
}

/// Open a Handle to mshv. Returns a handle to mshv or a `Handle` to an error
/// if there was an issue.
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
pub unsafe extern "C" fn open_mshv(ctx: *mut Context, require_stable_api: bool) -> Handle {
    match impls::open_mshv(require_stable_api) {
        Ok(mshv) => Context::register(mshv, &(*ctx).mshvs, Hdl::Mshv),
        Err(e) => (*ctx).register_err(e),
    }
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
pub unsafe extern "C" fn create_vm(ctx: *mut Context, mshv_handle: Handle) -> Handle {
    let mshv = match get_mshv(&mut (*ctx), mshv_handle) {
        Ok(result) => result,
        Err(e) => return (*ctx).register_err(e),
    };

    match impls::create_vm(&mshv) {
        Ok(vmfd) => Context::register(vmfd, &(*ctx).vmfds, Hdl::VmFd),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Create a vCPU and return a Handle to it. Returns a handle to a vCPU or a `Handle` to an error
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
/// - Used to call `create_vm`
///
/// 2. `Handle` to a `VmFd` that has been:
/// - Created with `create_vm`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API

#[no_mangle]
pub unsafe extern "C" fn create_vcpu(ctx: *mut Context, vmfd_handle: Handle) -> Handle {
    let vmfd = match get_vmfd(&mut (*ctx), vmfd_handle) {
        Ok(result) => result,
        Err(e) => return (*ctx).register_err(e),
    };

    match impls::create_vcpu(&vmfd) {
        Ok(vcpu) => Context::register(vcpu, &(*ctx).vcpufds, Hdl::VcpuFd),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Map a memory region in the host to the VM and return a Handle to it. Returns a handle to a mshv_user_mem_region or a `Handle` to an error
/// if there was an issue.
///
/// # Safety
///
/// You must destory this handle by calling `unmap_memory_region` exactly once
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
pub unsafe extern "C" fn map_vm_memory_region(
    ctx: *mut Context,
    vmfd_handle: Handle,
    guest_pfn: u64,
    load_address: u64,
    size: u64,
) -> Handle {
    let vmfd = match get_vmfd(&mut (*ctx), vmfd_handle) {
        Ok(result) => result,
        Err(e) => return (*ctx).register_err(e),
    };

    match impls::map_vm_memory_region(&vmfd, guest_pfn, load_address, size) {
        Ok(user_mem_region) => Context::register(
            user_mem_region,
            &(*ctx).mshv_user_mem_regions,
            Hdl::MshvUserMemRegion,
        ),
        Err(e) => (*ctx).register_err(e),
    }
}

/// Unmap a memory region in the host to the VM and return a Handle to it. Returns an empty handle or a `Handle` to an error
/// if there was an issue.
///
/// # Safety
///
/// If the handle is a Handle to an error then it should be freed by calling `handle_free` .The empty handle does not need to be freed but calling `handle_free` is will not cause an error.
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
///

#[no_mangle]
pub unsafe extern "C" fn unmap_vm_memory_region(
    ctx: *mut Context,
    vmfd_handle: Handle,
    mshv_user_mem_regions_handle: Handle,
) -> Handle {
    let vmfd = match get_vmfd(&mut (*ctx), vmfd_handle) {
        Ok(result) => result,
        Err(e) => return (*ctx).register_err(e),
    };

    let user_memory_region =
        match get_mshv_user_mem_region(&mut (*ctx), mshv_user_mem_regions_handle) {
            Ok(result) => result,
            Err(e) => return (*ctx).register_err(e),
        };

    match impls::unmap_vm_memory_region(&vmfd, &user_memory_region) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// mshv_register represents a register in the VM. It is used to set and get register values in the VM.

#[repr(C)]
#[derive(Copy, Clone)]
pub struct mshv_register {
    /// The name of the register - should be equal to one of the constant values with the prefix `HV_X64_REGISTER_`.
    pub name: c_uint,
    /// reserved1 should always be set to 0.
    pub reserved1: c_uint,
    /// reserved2 should always be set to 0.
    pub reserved2: c_ulonglong,
    /// The value of the register.
    pub value: mshv_u128,
}

/// mshv_u128 represents the value of a register.

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct mshv_u128 {
    /// The lower 64 bits of the register value.
    pub low_part: c_ulonglong,
    /// The upper 64 bits of the register value.
    pub high_part: c_ulonglong,
}

/// Set Registers in the vCPU. Returns an empty handle or a `Handle` to an error
/// if there was an issue.
///
/// # Safety
///
/// If the handle is a Handle to an error then it should be freed by calling `handle_free` .The empty handle does not need to be freed but calling `handle_free` is will not cause an error.
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
/// - Used to call `create_vcpu`
///
/// 2. `Handle` to a `VcpuFd` that has been:
/// - Created with `create_vcpu`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
///
/// 3. An array of `mshv_register`s
/// 4. The number of `mshv_register`s in the array
///

#[no_mangle]
pub unsafe extern "C" fn set_registers(
    ctx: *mut Context,
    vcpufd_handle: Handle,
    reg_ptr: *const mshv_register,
    reg_length: usize,
) -> Handle {
    let vcpufd = match get_vcpufd(&mut (*ctx), vcpufd_handle) {
        Ok(result) => result,
        Err(e) => return (*ctx).register_err(e),
    };

    let did_it_panic = catch_unwind(|| {
        let regs: &[mshv_register] = slice::from_raw_parts(reg_ptr, reg_length);
        regs
    });

    let ffi_regs = match did_it_panic {
        Ok(result) => result,
        Err(_) => {
            return (*ctx).register_err(anyhow::anyhow!(
                "failed to create array from reg_ptr ad reg_length"
            ))
        }
    };

    let mut regs: Vec<hv_register_assoc> = Vec::with_capacity(reg_length);

    for reg in ffi_regs {
        let hv_reg = hv_register_assoc {
            name: reg.name,
            value: hv_register_value {
                reg128: hv_u128 {
                    low_part: reg.value.low_part,
                    high_part: reg.value.high_part,
                },
            },
            reserved1: reg.reserved1,
            reserved2: reg.reserved2,
        };
        regs.push(hv_reg);
    }

    match impls::set_registers(&vcpufd, &regs) {
        Ok(_) => Handle::new_empty(),
        Err(e) => (*ctx).register_err(e),
    }
}

/// mshv_run_message contains the results of a vCPU execution
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct mshv_run_message {
    /// The exit reason of the vCPU.
    pub message_type: c_uint,
    /// The value of the RAX register.
    pub rax: u64,
    /// The value of the RIP register.
    pub rip: u64,
    /// The port number when the reason is hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT.
    pub port_number: u16,
    /// The size of the instruction. This is combined with the value of the RIP register to determine the next instruction to be executed.
    pub instruction_length: u32,
}

/// Unmapped Memory Access
pub const hv_message_type_HVMSG_UNMAPPED_GPA: hv_message_type = 2147483648;
/// Port IO (out called in the guest)
pub const hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT: hv_message_type = 2147549184;
/// HALT  (hlt called in the guest)
pub const hv_message_type_HVMSG_X64_HALT: hv_message_type = 2147549191;

/// Runs a vCPU. Returns an handle to an `mshv_run_message` or a `Handle` to an error
/// if there was an issue.
///
/// # Safety
///
/// If the handle is a Handle to an error then it should be freed by calling `handle_free`. If the handle is a valid handle to an `mshv_run_message` the corresponding `mshv_run_message`
/// should be retrieved using .
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
/// - Used to call `create_vcpu`
/// - Used to call `set_registers`
///
/// 2. `Handle` to a `VcpuFd` that has been:
/// - Created with `create_vcpu`
/// - Not yet freed with `handle_free`
/// - Not modified, except by calling functions in the Hyperlight C API
///
///

#[no_mangle]
pub unsafe extern "C" fn run_vcpu(ctx: *mut Context, vcpufd_handle: Handle) -> Handle {
    let vcpufd = match get_vcpufd(&mut (*ctx), vcpufd_handle) {
        Ok(result) => result,
        Err(e) => return (*ctx).register_err(e),
    };

    match impls::run_vcpu(&vcpufd) {
        Ok(run_result) => {
            let mut result = mshv_run_message {
                message_type: run_result.header.message_type,
                ..Default::default()
            };
            if result.message_type == hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT {
                let io_message = run_result.to_ioport_info().unwrap();
                result.port_number = io_message.port_number;
                result.rax = io_message.rax;
                result.rip = io_message.header.rip;
                result.instruction_length = io_message.header.instruction_length() as u32;
            };
            Context::register(result, &(*ctx).mshv_run_messages, Hdl::MshvRunMessage)
        }
        Err(e) => (*ctx).register_err(e),
    }
}

/// Gets the `mshv_run_message` associated with the given handle and frees the handle.
///
/// # Safety
///
/// The returned `mshv_run_message` should be freed by the caller when it is no longer needed.
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
/// - Used to call `create_vcpu`
/// - Used to call `set_registers`
/// - Used to call `run_vcpu`
///
/// 2. `Handle` to a `mshv_run_message` that has been:
/// - Created with `run_vcpu`
/// - Not yet used to call this function
/// - Not modified, except by calling functions in the Hyperlight C API
///
///

#[no_mangle]
pub unsafe extern "C" fn get_run_result_from_handle(
    ctx: *mut Context,
    handle: Handle,
) -> *const mshv_run_message {
    let result = match get_mshv_run_message(&mut (*ctx), handle) {
        Ok(result) => result,
        Err(_) => return std::ptr::null(),
    };

    Box::into_raw(Box::new(*result))
}

// TODO: should these be moved context?
fn get_mshv(ctx: &mut Context, handle: Handle) -> ReadResult<Mshv> {
    Context::get(handle, &ctx.mshvs, |b| matches!(b, Hdl::Mshv(_)))
}

fn get_vmfd(ctx: &mut Context, handle: Handle) -> ReadResult<VmFd> {
    Context::get(handle, &ctx.vmfds, |b| matches!(b, Hdl::VmFd(_)))
}

fn get_vcpufd(ctx: &mut Context, handle: Handle) -> ReadResult<VcpuFd> {
    Context::get(handle, &ctx.vcpufds, |b| matches!(b, Hdl::VcpuFd(_)))
}

fn get_mshv_user_mem_region(ctx: &mut Context, handle: Handle) -> ReadResult<mshv_user_mem_region> {
    Context::get(handle, &ctx.mshv_user_mem_regions, |b| {
        matches!(b, Hdl::MshvUserMemRegion(_))
    })
}

fn get_mshv_run_message(ctx: &mut Context, handle: Handle) -> ReadResult<mshv_run_message> {
    Context::get(handle, &ctx.mshv_run_messages, |b| {
        matches!(b, Hdl::MshvRunMessage(_))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc::c_void;
    use mshv_bindings::{hv_message, hv_message_type_HVMSG_X64_HALT, hv_register_name};
    use once_cell::sync::Lazy;
    use serde::Deserialize;
    use std::env;
    use std::io::Write;
    static TEST_CONFIG: Lazy<TestConfig> = Lazy::new(|| match envy::from_env::<TestConfig>() {
        Ok(config) => config,
        Err(err) => panic!("error parsing config from env: {}", err),
    });
    static SHOULD_RUN_TEST: Lazy<bool> = Lazy::new(is_hyperv_present);

    macro_rules! should_run_test {
        () => {{
            if !(*SHOULD_RUN_TEST) {
                println! {"Not Running Test SHOULD_RUN_TEST is false"}
                return;
            }
            println! {"Running Test SHOULD_RUN_TEST is true"}
        }};
    }

    fn hyperv_should_be_present_default() -> bool {
        false
    }

    fn should_have_stable_api_default() -> bool {
        false
    }

    #[derive(Deserialize, Debug)]
    struct TestConfig {
        #[serde(default = "hyperv_should_be_present_default")]
        // Set env var HYPERV_SHOULD_BE_PRESENT to require hyperv to be present for the tests.
        hyperv_should_be_present: bool,
        #[serde(default = "should_have_stable_api_default")]
        // Set env var SHOULD_HAVE_STABLE_API to require a stable api for the tests.
        should_have_stable_api: bool,
    }

    #[test]
    fn test_is_hypervisor_present() {
        let result = impls::is_hypervisor_present(true).unwrap_or(false);
        assert_eq!(
            result,
            TEST_CONFIG.hyperv_should_be_present && TEST_CONFIG.should_have_stable_api
        );
        assert!(!result);
        let result = impls::is_hypervisor_present(false).unwrap_or(false);
        assert_eq!(result, TEST_CONFIG.hyperv_should_be_present);
    }

    fn is_hyperv_present() -> bool {
        println!("SHOULD_HAVE_STABLE_API is {}", TEST_CONFIG.should_have_stable_api);
        println!("HYPERV_SHOULD_BE_PRESENT is {}", TEST_CONFIG.hyperv_should_be_present);
        impls::is_hypervisor_present(TEST_CONFIG.should_have_stable_api).unwrap_or(false)
    }

    #[test]
    fn test_open_mshv() {
        should_run_test!();
        let mshv = impls::open_mshv(TEST_CONFIG.should_have_stable_api);
        assert!(mshv.is_ok());
    }

    #[test]
    fn test_create_vm() {
        should_run_test!();
        let mshv = impls::open_mshv(TEST_CONFIG.should_have_stable_api);
        assert!(mshv.is_ok());
        let mshv = mshv.unwrap();
        let vmfd = impls::create_vm(&mshv);
        assert!(vmfd.is_ok());
    }

    #[test]
    fn test_create_vcpu() {
        should_run_test!();
        let mshv = impls::open_mshv(TEST_CONFIG.should_have_stable_api);
        assert!(mshv.is_ok());
        let mshv = mshv.unwrap();
        let vmfd = impls::create_vm(&mshv);
        assert!(vmfd.is_ok());
        let vmfd = vmfd.unwrap();
        let vcpu = impls::create_vcpu(&vmfd);
        assert!(vcpu.is_ok());
    }

    #[test]
    fn test_map_user_memory_region() {
        should_run_test!();
        let mshv = impls::open_mshv(TEST_CONFIG.should_have_stable_api);
        assert!(mshv.is_ok());
        let mshv = mshv.unwrap();
        let vmfd = impls::create_vm(&mshv);
        assert!(vmfd.is_ok());
        let vmfd = vmfd.unwrap();
        let vcpu = impls::create_vcpu(&vmfd);
        assert!(vcpu.is_ok());
        let guest_pfn = 0x1;
        let mem_size = 0x1000;
        let load_addr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                mem_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
                -1,
                0,
            )
        } as *mut u8;
        let user_memory_region =
            impls::map_vm_memory_region(&vmfd, guest_pfn, load_addr as u64, mem_size as u64);
        assert!(user_memory_region.is_ok());
        let user_memory_region = user_memory_region.unwrap();
        let result = impls::unmap_vm_memory_region(&vmfd, &user_memory_region);
        assert!(result.is_ok());
        unsafe { libc::munmap(load_addr as *mut c_void, mem_size) };
    }

    #[test]
    fn test_set_registers() {
        should_run_test!();
        let mshv = impls::open_mshv(TEST_CONFIG.should_have_stable_api);
        assert!(mshv.is_ok());
        let mshv = mshv.unwrap();
        let vmfd = impls::create_vm(&mshv);
        assert!(vmfd.is_ok());
        let vmfd = vmfd.unwrap();
        let vcpu = impls::create_vcpu(&vmfd);
        assert!(vcpu.is_ok());
        let vcpu = vcpu.unwrap();

        let regs = &[
            hv_register_assoc {
                name: hv_register_name::HV_X64_REGISTER_RAX as u32,
                value: hv_register_value { reg64: 12 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name::HV_X64_REGISTER_RBX as u32,
                value: hv_register_value { reg64: 24 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name::HV_X64_REGISTER_RFLAGS as u32,
                value: hv_register_value { reg64: 0x2000 },
                ..Default::default()
            },
        ];

        let result = impls::set_registers(&vcpu, regs);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_vcpu() {
        should_run_test!();
        let mshv = impls::open_mshv(TEST_CONFIG.should_have_stable_api);
        assert!(mshv.is_ok());
        let mshv = mshv.unwrap();
        let vmfd = impls::create_vm(&mshv);
        assert!(vmfd.is_ok());
        let vmfd = vmfd.unwrap();
        let vcpu = impls::create_vcpu(&vmfd);
        assert!(vcpu.is_ok());
        let vcpu = vcpu.unwrap();
        #[rustfmt::skip]
        let code:[u8;12] = [
           0xba, 0xf8, 0x03,  /* mov $0x3f8, %dx */
           0x00, 0xd8,         /* add %bl, %al */
           0x04, b'0',         /* add $'0', %al */
           0xee,               /* out %al, (%dx) */
           /* send a 0 to indicate we're done */
           0xb0, b'\0',        /* mov $'\0', %al */
           0xee,               /* out %al, (%dx) */
           0xf4, /* HLT */
        ];
        let guest_pfn = 0x1;
        let mem_size = 0x1000;
        let load_addr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                mem_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
                -1,
                0,
            )
        } as *mut u8;
        let user_memory_region =
            impls::map_vm_memory_region(&vmfd, guest_pfn, load_addr as u64, mem_size as u64);
        assert!(user_memory_region.is_ok());
        let user_memory_region = user_memory_region.unwrap();

        unsafe {
            let mut mslice = ::std::slice::from_raw_parts_mut(
                user_memory_region.userspace_addr as *mut u8,
                mem_size,
            );
            mslice.write_all(&code).unwrap();
        }

        let regs = &[
            hv_register_assoc {
                name: hv_register_name::HV_X64_REGISTER_CS as u32,
                value: hv_register_value {
                    reg128: hv_u128 {
                        low_part: 0,
                        high_part: 43628621390217215,
                    },
                },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name::HV_X64_REGISTER_RAX as u32,
                value: hv_register_value { reg64: 6 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name::HV_X64_REGISTER_RBX as u32,
                value: hv_register_value { reg64: 2 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name::HV_X64_REGISTER_RIP as u32,
                value: hv_register_value { reg64: 0x1000 },
                ..Default::default()
            },
            hv_register_assoc {
                name: hv_register_name::HV_X64_REGISTER_RFLAGS as u32,
                value: hv_register_value { reg64: 0x2 },
                ..Default::default()
            },
        ];

        let result = impls::set_registers(&vcpu, regs);
        assert!(result.is_ok());

        let run_result = impls::run_vcpu(&vcpu);
        assert!(run_result.is_ok());
        let run_message = run_result.unwrap();
        let message_type = run_message.header.message_type;

        assert_eq!(message_type, hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT);

        let io_message = run_message.to_ioport_info().unwrap();
        assert!(io_message.rax == b'8' as u64);
        assert!(io_message.port_number == 0x3f8);

        let regs = &[hv_register_assoc {
            name: hv_register_name::HV_X64_REGISTER_RIP as u32,
            value: hv_register_value {
                reg64: io_message.header.rip + io_message.header.instruction_length() as u64,
            },
            ..Default::default()
        }];

        let result = impls::set_registers(&vcpu, regs);
        assert!(result.is_ok());

        let run_result = impls::run_vcpu(&vcpu);
        assert!(run_result.is_ok());
        let run_message = run_result.unwrap();

        let message_type = run_message.header.message_type;

        let io_message = run_message.to_ioport_info().unwrap();
        assert_eq!(message_type, hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT);
        assert!(io_message.rax == b'\0' as u64);
        assert!(io_message.port_number == 0x3f8);

        let regs = &[hv_register_assoc {
            name: hv_register_name::HV_X64_REGISTER_RIP as u32,
            value: hv_register_value {
                reg64: io_message.header.rip + io_message.header.instruction_length() as u64,
            },
            ..Default::default()
        }];

        let result = impls::set_registers(&vcpu, regs);
        assert!(result.is_ok());

        let run_result = impls::run_vcpu(&vcpu);
        assert!(run_result.is_ok());
        let run_message = run_result.unwrap();

        let message_type = run_message.header.message_type;

        assert_eq!(message_type, hv_message_type_HVMSG_X64_HALT);

        let result = impls::unmap_vm_memory_region(&vmfd, &user_memory_region);
        assert!(result.is_ok());
        unsafe { libc::munmap(load_addr as *mut c_void, mem_size) };
    }
}
