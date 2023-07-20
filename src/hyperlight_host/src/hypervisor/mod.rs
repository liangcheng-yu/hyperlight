use anyhow::Result;
/// Handlers for Hypervisor custom logic
pub(crate) mod handlers;
#[cfg(target_os = "linux")]
/// HyperV-on-linux functionality
pub(crate) mod hyperv_linux;
#[cfg(target_os = "linux")]
/// Hypervisor-generic memory utilities
pub(crate) mod hypervisor_mem;
#[cfg(target_os = "linux")]
#[allow(dead_code)] // TODO: remove this when we have a working Rust sandbox
/// Functionality to manipulate KVM-based virtual machines
pub(crate) mod kvm;
#[cfg(target_os = "windows")]
/// Hyperlight Surrogate Process
pub(crate) mod surrogate_process;
#[cfg(target_os = "windows")]
/// Hyperlight Surrogate Process
pub(crate) mod surrogate_process_manager;

use self::handlers::{MemAccessHandlerRc, OutBHandlerRc};
use crate::mem::ptr::RawPtr;

/// A common set of hypervisor functionality
pub(crate) trait Hypervisor {
    /// Initialise the internally stored vCPU with the given PEB address and
    /// random number seed, then run it until a HLT instruction.
    fn initialise(
        &mut self,
        peb_addr: RawPtr,
        seed: u64,
        page_size: u32,
        outb_handle_fn: OutBHandlerRc,
        mem_access_fn: MemAccessHandlerRc,
    ) -> Result<()>;

    /// Run the internally stored vCPU until a HLT instruction.
    fn execute_until_halt(
        &mut self,
        outb_handle_fn: OutBHandlerRc,
        mem_access_fn: MemAccessHandlerRc,
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
        outb_handle_fn: OutBHandlerRc,
        mem_access_fn: MemAccessHandlerRc,
    ) -> Result<()>;

    /// Reset the stack pointer on the internal virtual CPU
    fn reset_rsp(&mut self, rsp: u64) -> Result<()>;
}

#[cfg(target_os = "linux")]
#[cfg(test)]
pub(crate) mod tests {
    use super::{
        handlers::{MemAccessHandlerRc, OutBHandlerRc},
        Hypervisor,
    };
    use crate::sandbox::mem_mgr::MemMgr;
    use crate::{
        mem::{
            layout::SandboxMemoryLayout,
            mgr::SandboxMemoryManager,
            ptr::{GuestPtr, RawPtr},
            ptr_offset::Offset,
        },
        sandbox::UninitializedSandbox,
        testing::dummy_guest_path,
    };
    use anyhow::bail;
    use anyhow::{anyhow, Result};
    use std::path::Path;

    pub(crate) fn test_initialise<NewFn>(
        outb_hdl: OutBHandlerRc,
        mem_access_hdl: MemAccessHandlerRc,
        new_fn: NewFn,
    ) -> Result<()>
    where
        NewFn: Fn(&SandboxMemoryManager, GuestPtr, GuestPtr) -> Result<Box<dyn Hypervisor>>,
    {
        let filename = dummy_guest_path()?;
        if !Path::new(&filename).exists() {
            bail!("test_initialise: file {} does not exist", filename);
        }

        let sandbox = UninitializedSandbox::new(filename.clone(), None, None, None)?;
        let mut mem_mgr = sandbox.get_mem_mgr().clone();
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
        let mut hypervisor_impl = new_fn(&mem_mgr, rsp_ptr, pml4_ptr)?;

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
