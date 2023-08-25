use anyhow::Result;
/// Handlers for Hypervisor custom logic
pub mod handlers;
/// HyperV-on-linux functionality
#[cfg(target_os = "linux")]
pub mod hyperv_linux;
#[cfg(target_os = "windows")]
/// Hyperv-on-windows functionality
pub(crate) mod hyperv_windows;
#[cfg(target_os = "linux")]
/// Hypervisor-generic memory utilities
pub mod hypervisor_mem;
#[cfg(target_os = "linux")]
/// Functionality to manipulate KVM-based virtual machines
pub mod kvm;
#[cfg(target_os = "windows")]
/// Hyperlight Surrogate Process
pub(crate) mod surrogate_process;
#[cfg(target_os = "windows")]
/// Hyperlight Surrogate Process
pub(crate) mod surrogate_process_manager;
/// WindowsHypervisorPlatform utilities
#[cfg(target_os = "windows")]
pub(crate) mod windows_hypervisor_platform;
/// Safe wrappers around windows types like `PSTR`
#[cfg(target_os = "windows")]
mod wrappers;

use self::handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper};
use crate::mem::ptr::RawPtr;
use std::fmt::Debug;

pub(crate) const CR4_PAE: u64 = 1 << 5;
pub(crate) const CR4_OSFXSR: u64 = 1 << 9;
pub(crate) const CR4_OSXMMEXCPT: u64 = 1 << 10;
pub(crate) const CR0_PE: u64 = 1;
pub(crate) const CR0_MP: u64 = 1 << 1;
pub(crate) const CR0_ET: u64 = 1 << 4;
pub(crate) const CR0_NE: u64 = 1 << 5;
pub(crate) const CR0_WP: u64 = 1 << 16;
pub(crate) const CR0_AM: u64 = 1 << 18;
pub(crate) const CR0_PG: u64 = 1 << 31;
pub(crate) const EFER_LME: u64 = 1 << 8;
pub(crate) const EFER_LMA: u64 = 1 << 10;

/// A common set of hypervisor functionality
pub trait Hypervisor: Debug + Sync + Send {
    /// Initialise the internally stored vCPU with the given PEB address and
    /// random number seed, then run it until a HLT instruction.
    fn initialise(
        &mut self,
        peb_addr: RawPtr,
        seed: u64,
        page_size: u32,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
    ) -> Result<()>;

    /// Run the internally stored vCPU until a HLT instruction.
    fn execute_until_halt(
        &mut self,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
    ) -> Result<()>;

    /// Dispatch a call from the host to the guest using the given pointer
    /// to the dispatch function _in the guest's address space_.
    ///
    /// Do this by setting the instruction pointer to `dispatch_func_addr`
    /// and then running the execution loop until a halt instruction.
    ///
    /// Returns `Ok` if the call succeeded, and an `Err` if it failed
    fn dispatch_call_from_host(
        &mut self,
        dispatch_func_addr: RawPtr,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
    ) -> Result<()>;

    /// Reset the stack pointer on the internal virtual CPU
    fn reset_rsp(&mut self, rsp: u64) -> Result<()>;

    /// Get the value of the stack pointer (RSP register) when this
    /// `Hypervisor` was first created
    fn orig_rsp(&self) -> Result<u64>;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper},
        Hypervisor,
    };
    use crate::{
        mem::{
            layout::SandboxMemoryLayout,
            mgr::SandboxMemoryManager,
            ptr::{GuestPtr, RawPtr},
            ptr_offset::Offset,
        },
        sandbox::{mem_mgr::MemMgrWrapperGetter, uninitialized::GuestBinary, UninitializedSandbox},
        testing::dummy_guest_path,
    };
    use anyhow::bail;
    use anyhow::{anyhow, Result};
    use std::path::Path;

    pub(crate) fn test_initialise<NewFn>(
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
        new_fn: NewFn,
    ) -> Result<()>
    where
        NewFn: Fn(&SandboxMemoryManager, GuestPtr, GuestPtr) -> Result<Box<dyn Hypervisor>>,
    {
        let filename = dummy_guest_path()?;
        if !Path::new(&filename).exists() {
            bail!("test_initialise: file {} does not exist", filename);
        }

        let mut sandbox =
            UninitializedSandbox::new(GuestBinary::FilePath(filename.clone()), None, None)?;
        let mem_mgr = {
            let wrapper = sandbox.get_mem_mgr_wrapper_mut();
            wrapper.as_mut()
        };
        let shared_mem = &mem_mgr.shared_mem;
        let rsp_ptr = {
            let mem_size: u64 = shared_mem.mem_size().try_into()?;
            let u64_val = mem_mgr.set_up_hypervisor_partition(mem_size)?;
            let base_addr_u64 = u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?;
            let offset = Offset::from(u64_val - base_addr_u64);
            GuestPtr::try_from(offset)
        }?;
        let pml4_ptr = {
            let offset_u64 = u64::try_from(SandboxMemoryLayout::PML4_OFFSET)?;
            let offset = Offset::from(offset_u64);
            GuestPtr::try_from(offset)
        }?;
        let mut hypervisor_impl = new_fn(mem_mgr, rsp_ptr, pml4_ptr)?;

        // call initialise on the hypervisor implementation with specific values
        // for PEB (process environment block) address, seed and page size.
        //
        // these values are not actually used, they're just checked inside
        // the dummy guest, and if they don't match these values, the dummy
        // guest issues a write to an invalid memory address, which in turn
        // fails this test.
        //
        // in this test, we're not actually testing whether a guest can issue
        // memory operations, call functions, etc... - we're just testing
        // whether we can configure the shared memory region, load a binary
        // into it, and run the CPU to completion (e.g., a HLT interrupt)
        hypervisor_impl
            .initialise(
                RawPtr::from(0x230000),
                1234567890,
                4096,
                outb_hdl,
                mem_access_hdl,
            )
            .map_err(|e| anyhow!("Error running hypervisor against {} ({:?})", filename, e))
    }
}
