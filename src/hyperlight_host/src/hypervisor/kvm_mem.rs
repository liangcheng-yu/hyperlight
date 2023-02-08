use anyhow::{anyhow, Result};
use kvm_bindings::{__u64, kvm_userspace_memory_region};
use kvm_ioctls::VmFd;
use std::os::raw::c_void;

use crate::mem::shared_mem::SharedMemory;

/// Map a VM memory region on the VM referenced by `vmfd` using the
/// `guest_phys_addr` parameter as the guest physical address, the
/// `userspace_addr` as the pointer to shared memory, and `memory_size`
/// as the size of that memory.
///
/// Unless you are building a wrapper for a the Hyperlight C API,
/// you should prefer to use `map_vm_memory_region` instead. If you must call
/// this function, and it returns `Ok(mem_region)`, you must call
/// `unmap_vm_memory_region` and pass the contained `mem_region`
/// to free internal resources.
///
/// # Safety
///
/// `userspace_addr` must be a valid pointer to a region of memory
/// of `memory_size`. This memory must have been created with `mmap`
/// and should be freed with `munmap` after `unmap_vm_memory_region`
/// is called.
pub fn map_vm_memory_region_raw(
    vmfd: &VmFd,
    guest_phys_addr: u64,
    userspace_addr: *const c_void,
    memory_size: u64,
) -> Result<kvm_userspace_memory_region> {
    let mem_region = kvm_userspace_memory_region {
        slot: 0,
        flags: 0,
        guest_phys_addr,
        memory_size,
        userspace_addr: userspace_addr as __u64,
    };
    let set_res = unsafe { (*vmfd).set_user_memory_region(mem_region) };
    match set_res {
        Ok(_) => Ok(mem_region),
        Err(e) => Err(anyhow!(e)),
    }
}

/// Unmap the memory region referenced by `region` on the VM referenced
/// by `vmfd`.
///
/// This function sets the `memory_size` field on `mem_region` to
/// `0`.
///
/// Note: Unless you are building a wrapper for a the Hyperlight C API,
/// you should prefer to use `map_vm_memory_region` instead. That
/// function will return a `KVMMappedUserspaceMemoryRegion`, which
/// automatically calls this function when it's no longer needed.

pub fn unmap_vm_memory_region_raw(
    vmfd: &VmFd,
    mem_region: &mut kvm_userspace_memory_region,
) -> Result<()> {
    mem_region.memory_size = 0;
    let res = unsafe {
        (*vmfd)
            .set_user_memory_region(*mem_region)
            .map_err(|e| anyhow!(e))
    };
    res.map_err(|e| anyhow!(e))
}

/// Map a VM memory region on the VM referenced by `vmfd` using the
/// `guest_phys_addr` parameter as the guest physical address, the
/// `userspace_addr` as the pointer to shared memory, and `memory_size`
/// as the size of that memory.
///
/// If `Ok(reg)` is returned, `reg` will automatically unmap the memory
/// region that was mapped in this function when it goes out of scope.
/// Therefore, callers should ensure it stays in scope so the memory
/// stays mapped until after all VCPU runs are complete.
///
/// # Safety
///
/// `userspace_addr` must be a valid pointer to a region of memory
/// of `memory_size`. This memory must have been created with `mmap`
/// and should be freed with `munmap` after `unmap_vm_memory_region`
/// is called.
pub fn map_vm_memory_region<'a>(
    vmfd: &'a VmFd,
    guest_phys_addr: u64,
    shared_mem: &SharedMemory,
) -> Result<KVMMappedUserspaceMemoryRegion<'a>> {
    let userspace_addr = shared_mem.raw_ptr();
    let memory_size = shared_mem.mem_size() as __u64;
    let region = kvm_userspace_memory_region {
        slot: 0,
        flags: 0,
        guest_phys_addr,
        memory_size,
        userspace_addr: userspace_addr as __u64,
    };
    let set_res = unsafe { (*vmfd).set_user_memory_region(region) };
    match set_res {
        Ok(_) => Ok(KVMMappedUserspaceMemoryRegion { vmfd, region }),
        Err(e) => Err(anyhow!(e)),
    }
}

/// A wrapper for a mapped `kvm_userspace_memory_region` that
/// implements `Drop` so callers do not have to remember to
/// unmap the stored memory region by calling
/// `unmap_vm_memory_region_raw`.
pub struct KVMMappedUserspaceMemoryRegion<'a> {
    vmfd: &'a VmFd,
    region: kvm_userspace_memory_region,
}

impl<'a> Drop for KVMMappedUserspaceMemoryRegion<'a> {
    fn drop(&mut self) {
        let _ = unmap_vm_memory_region_raw(self.vmfd, &mut self.region);
    }
}
