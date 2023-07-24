use super::{
    handlers::{MemAccessHandlerRc, OutBHandlerRc},
    Hypervisor, CR0_AM, CR0_ET, CR0_MP, CR0_NE, CR0_PE, CR0_PG, CR0_WP, CR4_OSFXSR, CR4_OSXMMEXCPT,
    CR4_PAE, EFER_LMA, EFER_LME,
};

use crate::mem::ptr::RawPtr;
use anyhow::{bail, Result};
use core::ffi::c_void;

use super::windows_hypervisor_platform as whp;
use super::{surrogate_process::SurrogateProcess, surrogate_process_manager::*};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem::take;
use std::string::String;
use windows::Win32::System::Hypervisor::{
    WHvMapGpaRangeFlagExecute, WHvMapGpaRangeFlagRead, WHvMapGpaRangeFlagWrite, WHvX64RegisterCr0,
    WHvX64RegisterCr3, WHvX64RegisterCr4, WHvX64RegisterCs, WHvX64RegisterEfer, WHvX64RegisterR8,
    WHvX64RegisterRcx, WHvX64RegisterRdx, WHvX64RegisterRflags, WHvX64RegisterRip,
    WHvX64RegisterRsp, WHV_PARTITION_HANDLE, WHV_REGISTER_NAME, WHV_REGISTER_VALUE,
    WHV_RUN_VP_EXIT_CONTEXT, WHV_RUN_VP_EXIT_REASON, WHV_UINT128, WHV_UINT128_0,
};

/// Wrapper around WHV_REGISTER_NAME so we can impl
/// Hash on the struct.
#[derive(PartialEq, Eq)]
pub(super) struct WhvRegisterNameWrapper(pub WHV_REGISTER_NAME);

impl Hash for WhvRegisterNameWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0 .0.hash(state);
    }
}

/// A Hypervisor driver for HyperV-on-Windows.
pub struct HypervWindowsDriver {
    size: usize,
    partition_handle: WHV_PARTITION_HANDLE,
    surrogate_process: SurrogateProcess,
    source_address: *mut c_void,
    virtual_processor_created: bool,
    registers: HashMap<WhvRegisterNameWrapper, WHV_REGISTER_VALUE>,
}

impl HypervWindowsDriver {
    pub fn new(
        size: usize,
        source_address: *mut c_void,
        sandbox_base_address: u64,
        pml4_address: u64,
        entry_point: u64,
        rsp: u64,
    ) -> Result<Self> {
        match whp::is_hypervisor_present() {
            Ok(true) => (),
            Ok(false) => bail!("hypervisor not present"),
            Err(e) => bail!(e),
        }

        // create and setup hypervisor partition
        let partition_handle: WHV_PARTITION_HANDLE = whp::create_partition()?;
        whp::set_processor_count(&partition_handle, 1)?;
        whp::setup_partition(&partition_handle)?;

        // get a surrogate process
        let surrogate_process =
            get_surrogate_process_manager()?.get_surrogate_process(size, source_address)?;

        let whp_map_gpa_range_flags =
            WHvMapGpaRangeFlagRead | WHvMapGpaRangeFlagWrite | WHvMapGpaRangeFlagExecute;
        whp::map_gpa_range(
            &partition_handle,
            &surrogate_process.process_handle,
            source_address,
            sandbox_base_address,
            size,
            whp_map_gpa_range_flags,
        )?;

        whp::create_virtual_processor(&partition_handle)?;
        let virtual_processor_created = true;

        let mut registers = HashMap::new();

        // prime the registers we will set when to run the workload on a vcpu
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterCr3),
            WHV_REGISTER_VALUE {
                Reg64: pml4_address,
            },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterCr4),
            WHV_REGISTER_VALUE {
                Reg64: CR4_PAE | CR4_OSFXSR | CR4_OSXMMEXCPT,
            },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterCr0),
            WHV_REGISTER_VALUE {
                Reg64: CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_WP | CR0_AM | CR0_PG,
            },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterEfer),
            WHV_REGISTER_VALUE {
                Reg64: EFER_LME | EFER_LMA,
            },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterCs),
            WHV_REGISTER_VALUE {
                Reg128: WHV_UINT128 {
                    Anonymous: WHV_UINT128_0 {
                        Low64: (0),
                        High64: (0xa09b0008ffffffff),
                    },
                },
            },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterRflags),
            WHV_REGISTER_VALUE { Reg64: 0x0002 },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterRip),
            WHV_REGISTER_VALUE { Reg64: entry_point },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterRsp),
            WHV_REGISTER_VALUE { Reg64: rsp },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterR8),
            WHV_REGISTER_VALUE { Reg64: 0x0 },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterRdx),
            WHV_REGISTER_VALUE { Reg64: 0x0 },
        );
        registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterRcx),
            WHV_REGISTER_VALUE { Reg64: 0x0 },
        );

        Ok(Self {
            size,
            partition_handle,
            surrogate_process,
            source_address,
            virtual_processor_created,
            registers,
        })
    }

    #[inline]
    fn throw_exit_exception(&self, exit_reason: WHV_RUN_VP_EXIT_REASON) -> Result<()> {
        // get registers
        let register_names = self.registers.keys().map(|x| x.0).collect();
        let registers =
            whp::get_virtual_processor_registers(&self.partition_handle, &register_names)?;

        let mut error = String::new();
        error.push_str(&format!(
            "Did not receive a halt from Hypervisor as expected - Received {exit_reason:?}!\n"
        ));
        for (key, value) in registers.iter() {
            unsafe {
                // need access to a union field!
                error.push_str(&format!(
                    "  {:>4?} - 0x{:0>16X} {:0>16X}\n",
                    key.0, value.Reg128.Anonymous.High64, value.Reg128.Anonymous.Low64
                ));
            }
        }
        bail!(error);
    }
}

impl Hypervisor for HypervWindowsDriver {
    fn initialise(
        &mut self,
        peb_address: RawPtr,
        seed: u64,
        page_size: u32,
        outb_hdl: OutBHandlerRc,
        mem_access_hdl: MemAccessHandlerRc,
    ) -> Result<()> {
        self.registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterRcx),
            WHV_REGISTER_VALUE {
                Reg64: peb_address.into(),
            },
        );
        self.registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterRdx),
            WHV_REGISTER_VALUE { Reg64: seed },
        );
        self.registers.insert(
            WhvRegisterNameWrapper(WHvX64RegisterR8),
            WHV_REGISTER_VALUE { Reg32: page_size },
        );
        whp::set_virtual_processor_registers(&self.partition_handle, &self.registers)?;
        self.execute_until_halt(outb_hdl, mem_access_hdl)
    }

    fn execute_until_halt(
        &mut self,
        outb_hdl: OutBHandlerRc,
        mem_access_hdl: MemAccessHandlerRc,
    ) -> Result<()> {
        let bytes_written: Option<*mut usize> = None;
        let bytes_read: Option<*mut usize> = None;
        loop {
            // TODO optimise this
            // the following write to and read from process memory is required as we need to use
            // surrogate processes to allow more than one WHP Partition per process
            // see HyperVSurrogateProcessManager
            // this needs updating so that
            // 1. it only writes to memory that changes between usage
            // 2. memory is allocated in the process once and then only freed and reallocated if the
            // memory needs to grow.

            // - copy stuff to surrogate process
            unsafe {
                if !windows::Win32::System::Diagnostics::Debug::WriteProcessMemory(
                    self.surrogate_process.process_handle,
                    self.surrogate_process.allocated_address,
                    self.source_address,
                    self.size,
                    bytes_written,
                )
                .as_bool()
                {
                    let hresult = windows::Win32::Foundation::GetLastError();
                    bail!("WriteProcessMemory Error: {}", hresult.to_hresult());
                }
            }

            // - call WHvRunVirtualProcessor
            let exit_context: WHV_RUN_VP_EXIT_CONTEXT =
                whp::run_virtual_processor(&self.partition_handle)?;

            // - call read-process memory
            unsafe {
                if !windows::Win32::System::Diagnostics::Debug::ReadProcessMemory(
                    self.surrogate_process.process_handle,
                    self.surrogate_process.allocated_address,
                    self.source_address as *mut c_void,
                    self.size,
                    bytes_read,
                )
                .as_bool()
                {
                    let hresult = windows::Win32::Foundation::GetLastError();
                    bail!("ReadProcessMemory Error: {}", hresult.to_hresult())
                }

                match exit_context.ExitReason {
                    // WHvRunVpExitReasonX64IoPortAccess
                    WHV_RUN_VP_EXIT_REASON(2i32) => {
                        outb_hdl.call(exit_context.Anonymous.IoPortAccess.PortNumber, 0);

                        // Move rip forward to next instruction (size of current instruction is in lower byte of InstructionLength_Cr8_Reserverd)
                        let registers = HashMap::from([(
                            WhvRegisterNameWrapper(WHvX64RegisterRip),
                            WHV_REGISTER_VALUE {
                                Reg64: (exit_context.VpContext.Reserved & 0xF) as u64,
                            },
                        )]);
                        whp::set_virtual_processor_registers(&self.partition_handle, &registers)?;
                        continue;
                    }
                    // HvRunVpExitReasonX64Halt
                    WHV_RUN_VP_EXIT_REASON(8i32) => {
                        break;
                    }
                    // WHvRunVpExitReasonMemoryAccess
                    WHV_RUN_VP_EXIT_REASON(1i32) => {
                        mem_access_hdl.call();
                        return self.throw_exit_exception(exit_context.ExitReason);
                    }
                    _ => {
                        return self.throw_exit_exception(exit_context.ExitReason);
                    }
                }
            }
        }

        Ok(())
    }

    fn dispatch_call_from_host(
        &mut self,
        dispatch_func_addr: RawPtr,
        outb_hdl: OutBHandlerRc,
        mem_access_hdl: MemAccessHandlerRc,
    ) -> Result<()> {
        let registers = HashMap::from([(
            WhvRegisterNameWrapper(WHvX64RegisterRip),
            WHV_REGISTER_VALUE {
                Reg64: dispatch_func_addr.into(),
            },
        )]);
        whp::set_virtual_processor_registers(&self.partition_handle, &registers)?;
        self.execute_until_halt(outb_hdl, mem_access_hdl)
    }

    fn reset_rsp(&mut self, rsp: u64) -> Result<()> {
        let registers = HashMap::from([(
            WhvRegisterNameWrapper(WHvX64RegisterRsp),
            WHV_REGISTER_VALUE { Reg64: rsp },
        )]);
        whp::set_virtual_processor_registers(&self.partition_handle, &registers)?;
        Ok(())
    }
}

impl Drop for HypervWindowsDriver {
    fn drop(&mut self) {
        let sp = take(&mut self.surrogate_process);
        let _result = get_surrogate_process_manager()
            .unwrap()
            .return_surrogate_process(sp); // TODO error handling

        if self.virtual_processor_created {
            whp::delete_virtual_process(&self.partition_handle).unwrap();
        }

        if self.partition_handle != WHV_PARTITION_HANDLE::default() {
            whp::delete_partition(&self.partition_handle).unwrap();
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        hypervisor::{
            handlers::{MemAccessHandlerFn, OutBHandlerFn},
            tests::test_initialise,
        },
        mem::{layout::SandboxMemoryLayout, ptr::GuestPtr, ptr_offset::Offset},
        testing::surrogate_binary::copy_surrogate_exe,
    };
    use serial_test::serial;
    use std::rc::Rc;

    use super::HypervWindowsDriver;

    extern "C" fn outb_fn(_port: u16, _payload: u64) {}
    extern "C" fn mem_access_fn() {}

    #[test]
    #[serial]
    fn test_init() {
        assert!(copy_surrogate_exe());

        let outb_handler = {
            let func: Box<dyn Fn(u16, u64)> = Box::new(|_, _| {});
            Rc::new(OutBHandlerFn::from(func))
        };
        let mem_access_handler = {
            let func: Box<dyn Fn()> = Box::new(|| {});
            Rc::new(MemAccessHandlerFn::from(func))
        };
        test_initialise(
            outb_handler,
            mem_access_handler,
            |mgr, rsp_ptr, pml4_ptr| {
                let host_addr = u64::try_from(mgr.shared_mem.base_addr())?;
                let rsp = rsp_ptr.absolute()?;
                let _guest_pfn = u64::try_from(SandboxMemoryLayout::BASE_ADDRESS << 12)?;
                let entrypoint = {
                    let load_addr = mgr.load_addr.clone();
                    let load_offset_u64 =
                        u64::from(load_addr) - u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?;
                    let total_offset = Offset::from(load_offset_u64) + mgr.entrypoint_offset;
                    GuestPtr::try_from(total_offset)
                }?;
                let driver = HypervWindowsDriver::new(
                    mgr.shared_mem.mem_size(),
                    host_addr as *mut core::ffi::c_void,
                    u64::try_from(SandboxMemoryLayout::BASE_ADDRESS)?,
                    pml4_ptr.absolute()?,
                    entrypoint.absolute().unwrap(),
                    rsp,
                )?;

                Ok(Box::new(driver))
            },
        )
        .unwrap();
    }
}
