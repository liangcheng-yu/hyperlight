use super::{
    handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper},
    Hypervisor, CR0_AM, CR0_ET, CR0_MP, CR0_NE, CR0_PE, CR0_PG, CR0_WP, CR4_OSFXSR, CR4_OSXMMEXCPT,
    CR4_PAE, EFER_LMA, EFER_LME,
};

use crate::{
    error::HyperlightError::HypervisorError, hypervisor::hypervisor_mem::HypervisorAddrs,
    mem::ptr::GuestPtr, new_error,
};
use crate::{hypervisor::HyperlightExit, mem::ptr::RawPtr};
use crate::{log_then_return, Result};
use log::error;
use mshv_bindings::{
    hv_message, hv_message_type, hv_message_type_HVMSG_UNMAPPED_GPA,
    hv_message_type_HVMSG_X64_HALT, hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT, hv_register_assoc,
    hv_register_name, hv_register_name_HV_X64_REGISTER_CR0, hv_register_name_HV_X64_REGISTER_CR3,
    hv_register_name_HV_X64_REGISTER_CR4, hv_register_name_HV_X64_REGISTER_CS,
    hv_register_name_HV_X64_REGISTER_EFER, hv_register_name_HV_X64_REGISTER_R8,
    hv_register_name_HV_X64_REGISTER_RAX, hv_register_name_HV_X64_REGISTER_RBX,
    hv_register_name_HV_X64_REGISTER_RCX, hv_register_name_HV_X64_REGISTER_RDX,
    hv_register_name_HV_X64_REGISTER_RFLAGS, hv_register_name_HV_X64_REGISTER_RIP,
    hv_register_name_HV_X64_REGISTER_RSP, hv_register_value, hv_u128, mshv_user_mem_region,
};
use mshv_ioctls::{Mshv, VcpuFd, VmFd};
use once_cell::sync::Lazy;
use std::{any::Any, env};
use std::{collections::HashMap, time::Duration};
use tracing::{instrument, Span};

/// Determine whether the HyperV for Linux hypervisor API is present
/// and functional. If `REQUIRE_STABLE_API` is true, determines only whether a
/// stable API for the Linux HyperV hypervisor is present.
#[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
//TODO:(#1029) Once CAPI is complete this does not need to be public
pub fn is_hypervisor_present() -> Result<bool> {
    let mshv = Mshv::new()?;
    match mshv.check_stable() {
        Ok(stable) => {
            if stable {
                Ok(true)
            } else {
                Ok(!*REQUIRE_STABLE_API)
            }
        }
        Err(e) => {
            log_then_return!(HypervisorError(e));
        }
    }
}
/// The constant to map guest physical addresses as readable
/// in an mshv memory region
const HV_MAP_GPA_READABLE: u32 = 1;
/// The constant to map guest physical addresses as writable
/// in an mshv memory region
const HV_MAP_GPA_WRITABLE: u32 = 2;
/// The constant to map guest physical addresses as executable
/// in an mshv memory region
const HV_MAP_GPA_EXECUTABLE: u32 = 12;

// TODO: Question should we make the default true (i.e. we only allow unstable API if the Env Var is set)
// The only reason the default is as it is now is because there is no stable API for hyperv on Linux
// But at some point a release will be made and this will seem backwards

static REQUIRE_STABLE_API: Lazy<bool> =
    Lazy::new(|| match env::var("HYPERV_SHOULD_HAVE_STABLE_API") {
        Ok(val) => val.parse::<bool>().unwrap_or(false),
        Err(_) => false,
    });

type RegistersHashMap = HashMap<hv_register_name, hv_register_value>;

/// A Hypervisor driver for HyperV-on-Linux. This hypervisor is often
/// called the Microsoft Hypervisor Platform (MSHV)
//TODO:(#1029) Once CAPI is complete this does not need to be public
pub struct HypervLinuxDriver {
    _mshv: Mshv,
    vm_fd: VmFd,
    vcpu_fd: VcpuFd,
    mem_region: mshv_user_mem_region,
    // note: we should use a HashSet here rather than this
    // HashMap, but to do that, hv_register_assoc needs to
    // implement Eq and PartialEq
    // since it implements neither, we have to use a HashMap
    // instead and use the registers's name -- a u32 -- as the key
    registers: RegistersHashMap,
    orig_rsp: GuestPtr,
}

impl std::fmt::Debug for HypervLinuxDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HypervLinuxDriver")
            .field("mem_region", &self.mem_region)
            .finish()
    }
}

impl HypervLinuxDriver {
    /// Create a new `HypervLinuxDriver`, complete with all registers
    /// set up to execute a Hyperlight binary inside a HyperV-powered
    /// sandbox on Linux.
    ///
    /// While registers are set up, they will not have been applied to
    /// the underlying virtual CPU after this function returns. Call the
    /// `apply_registers` method to do that, or more likely call
    /// `initialise` to do it for you.
    //TODO:(#1029) Once CAPI is complete this does not need to be public
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn new(addrs: &HypervisorAddrs, rsp_ptr: GuestPtr, pml4_ptr: GuestPtr) -> Result<Self> {
        match is_hypervisor_present() {
            Ok(true) => (),
            Ok(false) => {
                log_then_return!(
                    "Hypervisor not present (stable api was {:?})",
                    *REQUIRE_STABLE_API
                );
            }
            Err(e) => {
                log_then_return!(e);
            }
        }
        let mshv = Mshv::new()?;
        let pr = Default::default();
        let vm_fd = mshv.create_vm_with_config(&pr)?;
        let mut vcpu_fd = vm_fd.create_vcpu(0)?;
        let mem_region = mshv_user_mem_region {
            size: addrs.mem_size,
            guest_pfn: addrs.guest_pfn,
            userspace_addr: addrs.host_addr,
            flags: HV_MAP_GPA_READABLE | HV_MAP_GPA_WRITABLE | HV_MAP_GPA_EXECUTABLE,
        };

        vm_fd.map_user_memory(mem_region)?;
        let registers = {
            let mut hm = HashMap::new();
            Self::add_registers(&mut vcpu_fd, &mut hm, addrs, rsp_ptr.clone(), pml4_ptr)?;
            hm
        };
        Ok(Self {
            _mshv: mshv,
            vm_fd,
            vcpu_fd,
            mem_region,
            registers,
            orig_rsp: rsp_ptr,
        })
    }

    /// Add all register values to the pending list of registers, but do not
    /// apply them.
    ///
    /// If you want to manually apply registers to the stored vCPU, call
    /// `apply_registers`. `initialise` and `dispatch_call_from_host` will
    /// also do so automatically.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn add_registers(
        vcpu: &mut VcpuFd,
        registers: &mut RegistersHashMap,
        addrs: &HypervisorAddrs,
        rsp_ptr: GuestPtr,
        pml4_ptr: GuestPtr,
    ) -> Result<()> {
        // set CS register. adapted from:
        // https://github.com/rust-vmm/mshv/blob/ed66a5ad37b107c972701f93c91e8c7adfe6256a/mshv-ioctls/src/ioctls/vcpu.rs#L1165-L1169
        {
            // get CS Register
            let mut cs_reg = hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_CS,
                ..Default::default()
            };
            vcpu.get_reg(std::slice::from_mut(&mut cs_reg))?;
            cs_reg.value.segment.base = 0;
            cs_reg.value.segment.selector = 0;
            registers.insert(hv_register_name_HV_X64_REGISTER_CS, cs_reg.value);
        }

        registers.insert(
            hv_register_name_HV_X64_REGISTER_RAX,
            hv_register_value { reg64: 2 },
        );

        registers.insert(
            hv_register_name_HV_X64_REGISTER_RBX,
            hv_register_value { reg64: 2 },
        );

        registers.insert(
            hv_register_name_HV_X64_REGISTER_RFLAGS,
            hv_register_value { reg64: 0x2 },
        );

        registers.insert(
            hv_register_name_HV_X64_REGISTER_RIP,
            hv_register_value {
                reg64: addrs.entrypoint,
            },
        );

        registers.insert(
            hv_register_name_HV_X64_REGISTER_RSP,
            hv_register_value {
                reg64: rsp_ptr.absolute()?,
            },
        );

        registers.insert(
            hv_register_name_HV_X64_REGISTER_CR3,
            hv_register_value {
                reg64: pml4_ptr.absolute()?,
            },
        );

        registers.insert(
            hv_register_name_HV_X64_REGISTER_CR4,
            hv_register_value {
                reg64: CR4_PAE | CR4_OSFXSR | CR4_OSXMMEXCPT,
            },
        );

        registers.insert(
            hv_register_name_HV_X64_REGISTER_CR0,
            hv_register_value {
                reg64: CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_WP | CR0_AM | CR0_PG,
            },
        );

        registers.insert(
            hv_register_name_HV_X64_REGISTER_EFER,
            hv_register_value {
                reg64: EFER_LME | EFER_LMA,
            },
        );

        registers.insert(
            hv_register_name_HV_X64_REGISTER_CS,
            hv_register_value {
                reg128: hv_u128 {
                    low_part: 0,
                    high_part: 0xa09b0008ffffffff,
                },
            },
        );
        Ok(())
    }

    /// Apply the internally stored register list on the internally
    /// stored virtual CPU.
    ///
    /// Call `add_registers` prior to this function to add to the internal
    /// register list.
    //TODO:(#1029) Once CAPI is complete this does not need to be public
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn apply_registers(&self) -> Result<()> {
        let mut regs_vec: Vec<hv_register_assoc> = Vec::new();
        for (k, v) in &self.registers {
            regs_vec.push(hv_register_assoc {
                name: *k,
                value: *v,
                ..Default::default()
            });
        }

        Ok(self.vcpu_fd.set_reg(regs_vec.as_slice())?)
    }

    /// Update the rip register in the internally stored list of registers
    /// as well as directly on the vCPU.
    ///
    /// This function will not apply any other pending changes on
    /// the internal register list.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn update_rip(&mut self, val: RawPtr) -> Result<()> {
        self.update_register_u64(hv_register_name_HV_X64_REGISTER_RIP, val.into())
    }

    /// Update the value of a specific register in the internally stored
    /// virtual CPU, and store this register update in the pending list
    /// of registers
    ///
    /// This function will apply only the value of the given register on the
    /// internally stored virtual CPU, but no others in the pending list.
    //TODO:(#1029) Once CAPI is complete this does not need to be public
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn update_register_u64(&mut self, name: hv_register_name, val: u64) -> Result<()> {
        self.registers
            .insert(name, hv_register_value { reg64: val });
        let reg = hv_register_assoc {
            name,
            value: hv_register_value { reg64: val },
            ..Default::default()
        };
        Ok(self.vcpu_fd.set_reg(&[reg])?)
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn get_rsp(&self) -> Result<u64> {
        let mut rsp_reg = hv_register_assoc {
            name: hv_register_name_HV_X64_REGISTER_RSP,
            ..Default::default()
        };
        self.vcpu_fd.get_reg(std::slice::from_mut(&mut rsp_reg))?;
        Ok(unsafe { rsp_reg.value.reg64 })
    }
}

impl Hypervisor for HypervLinuxDriver {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn as_mut_hypervisor(&mut self) -> &mut dyn Hypervisor {
        self as &mut dyn Hypervisor
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn initialise(
        &mut self,
        peb_addr: RawPtr,
        seed: u64,
        page_size: u32,
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
        max_execution_time: Duration,
        max_wait_for_cancellation: Duration,
    ) -> Result<()> {
        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_RCX,
            hv_register_value {
                reg64: peb_addr.into(),
            },
        );
        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_RDX,
            hv_register_value { reg64: seed },
        );
        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_R8,
            hv_register_value { reg32: page_size },
        );
        self.apply_registers()?;
        self.execute_until_halt(
            outb_hdl,
            mem_access_hdl,
            max_execution_time,
            max_wait_for_cancellation,
        )?;
        // we need to reset the stack pointer once execution is complete
        // the caller is responsible for this in windows x86_64 calling convention and since we are "calling" here we need to reset it
        self.reset_rsp(self.orig_rsp()?)
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn handle_io(
        &mut self,
        port: u16,
        data: Vec<u8>,
        rip: u64,
        instruction_length: u64,
        outb_handle_fn: OutBHandlerWrapper,
    ) -> Result<()> {
        let payload = data[..8].try_into()?;
        outb_handle_fn
            .lock()
            .map_err(|e| new_error!("Error Locking {}", e))?
            .call(port, u64::from_le_bytes(payload))?;

        self.update_rip(RawPtr::from(rip + instruction_length))
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn run(&mut self) -> Result<super::HyperlightExit> {
        const HALT_MESSAGE: hv_message_type = hv_message_type_HVMSG_X64_HALT;
        const IO_PORT_INTERCEPT_MESSAGE: hv_message_type =
            hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT;
        const UNMAPPED_GPA_MESSAGE: hv_message_type = hv_message_type_HVMSG_UNMAPPED_GPA;

        let hv_message: hv_message = Default::default();
        let result = match &self.vcpu_fd.run(hv_message) {
            Ok(m) => match m.header.message_type {
                HALT_MESSAGE => HyperlightExit::Halt(),
                IO_PORT_INTERCEPT_MESSAGE => {
                    let io_message = m.to_ioport_info()?;
                    let port_number = io_message.port_number;
                    let rip = io_message.header.rip;
                    let rax = io_message.rax;
                    let instruction_length = io_message.header.instruction_length() as u64;

                    HyperlightExit::IoOut(
                        port_number,
                        rax.to_le_bytes().to_vec(),
                        rip,
                        instruction_length,
                    )
                }
                UNMAPPED_GPA_MESSAGE => {
                    let mimo_message = m.to_memory_info()?;
                    let addr = mimo_message.guest_physical_address;
                    HyperlightExit::Mmio(addr)
                }
                other => {
                    log_then_return!("unknown Hyper-V run message type {:?}", other);
                }
            },
            Err(e) => match e.errno() {
                // we send a signal to the thread to cancel execution this results in EINTR being returned by KVM so we return Cancelled
                libc::EINTR => HyperlightExit::Cancelled(),
                libc::EAGAIN => HyperlightExit::Retry(),
                _ => {
                    log_then_return!("Error running VCPU {:?}", e);
                }
            },
        };
        Ok(result)
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn dispatch_call_from_host(
        &mut self,
        dispatch_func_addr: RawPtr,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
        max_execution_time: Duration,
        max_wait_for_cancellation: Duration,
    ) -> Result<()> {
        self.update_rip(dispatch_func_addr)?;
        // we need to reset the stack pointer once execution is complete
        // the caller is responsible for this in windows x86_64 calling convention and since we are "calling" here we need to reset it
        // so here we get the current RSP value so we can reset it later
        let rsp = self.get_rsp()?;
        self.execute_until_halt(
            outb_handle_fn,
            mem_access_fn,
            max_execution_time,
            max_wait_for_cancellation,
        )?;
        // Reset the stack pointer to the value it was before the call
        self.reset_rsp(rsp)
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn reset_rsp(&mut self, rsp: u64) -> Result<()> {
        self.update_register_u64(hv_register_name_HV_X64_REGISTER_RSP, rsp)
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn orig_rsp(&self) -> Result<u64> {
        self.orig_rsp.absolute()
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for HypervLinuxDriver {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn drop(&mut self) {
        match self.vm_fd.unmap_user_memory(self.mem_region) {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to unmap user memory in HyperVOnLinux ({:?})", e)
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_cfg {
    use once_cell::sync::Lazy;
    use serde::Deserialize;

    pub(crate) static TEST_CONFIG: Lazy<TestConfig> =
        Lazy::new(|| match envy::from_env::<TestConfig>() {
            Ok(config) => config,
            Err(err) => panic!("error parsing config from env: {}", err),
        });
    pub(crate) static SHOULD_RUN_TEST: Lazy<bool> = Lazy::new(is_hyperv_present);

    fn is_hyperv_present() -> bool {
        println!(
            "HYPERV_SHOULD_HAVE_STABLE_API is {}",
            TEST_CONFIG.hyperv_should_have_stable_api
        );
        println!(
            "HYPERV_SHOULD_BE_PRESENT is {}",
            TEST_CONFIG.hyperv_should_be_present
        );
        let is_present = super::is_hypervisor_present().unwrap_or(false);
        if (is_present && !TEST_CONFIG.hyperv_should_be_present)
            || (!is_present && TEST_CONFIG.hyperv_should_be_present)
        {
            panic!(
                "WARNING Hyper-V is present returned  {}, should be present is: {} HYPERV_SHOULD_HAVE_STABLE_API is {}",
                is_present, TEST_CONFIG.hyperv_should_be_present, TEST_CONFIG.hyperv_should_have_stable_api
            );
        }
        is_present
    }
    fn hyperv_should_be_present_default() -> bool {
        false
    }

    fn hyperv_should_have_stable_api_default() -> bool {
        false
    }
    #[derive(Deserialize, Debug)]
    pub(crate) struct TestConfig {
        #[serde(default = "hyperv_should_be_present_default")]
        // Set env var HYPERV_SHOULD_BE_PRESENT to require hyperv to be present for the tests.
        pub(crate) hyperv_should_be_present: bool,
        #[serde(default = "hyperv_should_have_stable_api_default")]
        // Set env var HYPERV_SHOULD_HAVE_STABLE_API to require a stable api for the tests.
        pub(crate) hyperv_should_have_stable_api: bool,
    }

    #[macro_export]
    macro_rules! should_run_hyperv_linux_test {
        () => {{
            if !(*SHOULD_RUN_TEST) {
                println! {"Not Running Test SHOULD_RUN_TEST is false"}
                return;
            }
            println! {"Running Test SHOULD_RUN_TEST is true"}
        }};
    }
}
#[cfg(test)]
mod tests {
    use super::test_cfg::{SHOULD_RUN_TEST, TEST_CONFIG};
    use super::*;
    use crate::mem::ptr_offset::Offset;
    use crate::{mem::shared_mem::SharedMemory, should_run_hyperv_linux_test};

    #[rustfmt::skip]
    const CODE:[u8;12] = [
        0xba, 0xf8, 0x03,  /* mov $0x3f8, %dx */
        0x00, 0xd8,         /* add %bl, %al */
        0x04, b'0',         /* add $'0', %al */
        0xee,               /* out %al, (%dx) */
        /* send a 0 to indicate we're done */
        0xb0, b'\0',        /* mov $'\0', %al */
        0xee,               /* out %al, (%dx) */
        0xf4, /* HLT */
    ];
    fn shared_mem_with_code(
        code: &[u8],
        mem_size: usize,
        load_offset: Offset,
    ) -> Result<Box<SharedMemory>> {
        let load_offset_usize = usize::try_from(load_offset)?;
        if load_offset_usize > mem_size {
            log_then_return!(
                "code load offset ({}) > memory size ({})",
                u64::from(load_offset),
                mem_size
            );
        }
        let mut shared_mem = SharedMemory::new(mem_size)?;
        shared_mem.copy_from_slice(code, load_offset)?;
        Ok(Box::new(shared_mem))
    }

    #[test]
    fn is_hypervisor_present() {
        // TODO add test for HYPERV_SHOULD_HAVE_STABLE_API = true
        let result = super::is_hypervisor_present().unwrap_or(false);
        assert_eq!(result, TEST_CONFIG.hyperv_should_be_present);
    }

    #[test]
    fn create_driver() {
        should_run_hyperv_linux_test!();
        const MEM_SIZE: usize = 0x1000;
        let gm = shared_mem_with_code(CODE.as_slice(), MEM_SIZE, Offset::zero()).unwrap();
        let addrs = HypervisorAddrs::for_shared_mem(&gm, MEM_SIZE as u64, 0, 0).unwrap();
        let rsp_ptr = GuestPtr::try_from(Offset::from(0)).unwrap();
        let pml4_ptr = GuestPtr::try_from(Offset::from(0)).unwrap();
        super::HypervLinuxDriver::new(&addrs, rsp_ptr, pml4_ptr).unwrap();
    }
}
