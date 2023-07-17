use super::{handlers::MemAccessHandlerRc, Hypervisor};
use super::{
    handlers::OutBHandlerRc,
    hypervisor_mem::{
        CR0_AM, CR0_ET, CR0_MP, CR0_NE, CR0_PE, CR0_PG, CR0_WP, CR4_OSFXSR, CR4_OSXMMEXCPT,
        CR4_PAE, EFER_LMA, EFER_LME,
    },
};
use crate::hypervisor::hypervisor_mem::HypervisorAddrs;
use crate::mem::ptr::RawPtr;
use anyhow::{anyhow, bail, Result};
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
//use mshv_bindings::*;
use mshv_ioctls::{Mshv, VcpuFd, VmFd};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;

/// Determine whether the HyperV for Linux hypervisor API is present
/// and functional. If `REQUIRE_STABLE_API` is true, determines only whether a
/// stable API for the Linux HyperV hypervisor is present.
pub(crate) fn is_hypervisor_present() -> Result<bool> {
    let mshv = Mshv::new()?;
    match mshv.check_stable() {
        Ok(stable) => {
            if stable {
                Ok(true)
            } else {
                Ok(!*REQUIRE_STABLE_API)
            }
        }
        Err(e) => bail!(e),
    }
}
/// The constant to map guest physical addresses as readable
/// in an mshv memory region
pub(crate) const HV_MAP_GPA_READABLE: u32 = 1;
/// The constant to map guest physical addresses as writable
/// in an mshv memory region
pub(crate) const HV_MAP_GPA_WRITABLE: u32 = 2;
/// The constant to map guest physical addresses as executable
/// in an mshv memory region
pub(crate) const HV_MAP_GPA_EXECUTABLE: u32 = 12;

// TODO: Question should we make the default true (i.e. we only allow unstable API if the Env Var is set)
// The only reason the default is as it is now is because there is no stable API for hyperv on Linux
// But at some point a release will be made and this will seem backwards

static REQUIRE_STABLE_API: Lazy<bool> =
    Lazy::new(|| match env::var("HYPERV_SHOULD_HAVE_STABLE_API") {
        Ok(val) => val.parse::<bool>().unwrap_or(false),
        Err(_) => false,
    });

/// A Hypervisor driver for HyperV-on-Linux. This hypervisor is often
/// called the Microsoft Hypervisor Platform (MSHV)
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
    registers: HashMap<hv_register_name, hv_register_value>,
}

impl HypervLinuxDriver {
    /// Create a new instance of `Self`, without any registers
    /// set.
    ///
    /// Call `add_basic_registers` or `add_advanced_registers`,
    /// then `apply_registers` to do so.
    pub fn new(addrs: &HypervisorAddrs) -> Result<Self> {
        match is_hypervisor_present() {
            Ok(true) => (),
            Ok(false) => bail!(
                "Hypervisor not present (stable api was {:?})",
                *REQUIRE_STABLE_API
            ),
            Err(e) => bail!(e),
        }
        let mshv = Mshv::new().map_err(|e| anyhow!(e))?;
        let pr = Default::default();
        let vm_fd = mshv.create_vm_with_config(&pr).map_err(|e| anyhow!(e))?;
        let vcpu_fd = vm_fd.create_vcpu(0).map_err(|e| anyhow!(e))?;
        let mem_region = mshv_user_mem_region {
            size: addrs.mem_size,
            guest_pfn: addrs.guest_pfn,
            userspace_addr: addrs.host_addr,
            flags: HV_MAP_GPA_READABLE | HV_MAP_GPA_WRITABLE | HV_MAP_GPA_EXECUTABLE,
        };

        vm_fd.map_user_memory(mem_region).map_err(|e| anyhow!(e))?;
        Ok(Self {
            _mshv: mshv,
            vm_fd,
            vcpu_fd,
            mem_region,
            registers: HashMap::new(),
        })
    }

    /// Add basic registers to the pending list of registers, but do not
    /// apply them.
    ///
    /// The added registers will be suitable for running very "basic" code
    /// that uses no memory.
    pub fn add_basic_registers(&mut self, addrs: &HypervisorAddrs) -> Result<()> {
        // set CS register. adapted from:
        // https://github.com/rust-vmm/mshv/blob/ed66a5ad37b107c972701f93c91e8c7adfe6256a/mshv-ioctls/src/ioctls/vcpu.rs#L1165-L1169
        {
            // get CS Register
            let mut cs_reg = hv_register_assoc {
                name: hv_register_name_HV_X64_REGISTER_CS,
                ..Default::default()
            };
            self.vcpu_fd
                .get_reg(std::slice::from_mut(&mut cs_reg))
                .map_err(|e| anyhow!(e))?;
            cs_reg.value.segment.base = 0;
            cs_reg.value.segment.selector = 0;
            self.registers
                .insert(hv_register_name_HV_X64_REGISTER_CS, cs_reg.value);
        }

        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_RAX,
            hv_register_value { reg64: 2 },
        );

        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_RBX,
            hv_register_value { reg64: 2 },
        );

        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_RFLAGS,
            hv_register_value { reg64: 0x2 },
        );

        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_RIP,
            hv_register_value {
                reg64: addrs.entrypoint,
            },
        );
        Ok(())
    }

    /// Create an "advanced" version of `Self`, equipped to execute code that
    /// accesses memory
    pub fn add_advanced_registers(
        &mut self,
        addrs: &HypervisorAddrs,
        rsp: u64,
        pml4: u64,
    ) -> Result<()> {
        self.add_basic_registers(addrs)?;

        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_RSP,
            hv_register_value { reg64: rsp },
        );

        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_CR3,
            hv_register_value { reg64: pml4 },
        );

        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_CR4,
            hv_register_value {
                reg64: CR4_PAE | CR4_OSFXSR | CR4_OSXMMEXCPT,
            },
        );

        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_CR0,
            hv_register_value {
                reg64: CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_WP | CR0_AM | CR0_PG,
            },
        );

        self.registers.insert(
            hv_register_name_HV_X64_REGISTER_EFER,
            hv_register_value {
                reg64: EFER_LME | EFER_LMA,
            },
        );

        self.registers.insert(
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
    pub fn apply_registers(&self) -> Result<()> {
        let mut regs_vec: Vec<hv_register_assoc> = Vec::new();
        for (k, v) in &self.registers {
            regs_vec.push(hv_register_assoc {
                name: *k,
                value: *v,
                ..Default::default()
            });
        }

        self.vcpu_fd
            .set_reg(regs_vec.as_slice())
            .map_err(|e| anyhow!(e))
    }

    /// Update the rip register in the internally stored list of registers
    /// as well as directly on the vCPU.
    ///
    /// This function will not apply any other pending changes on
    /// the internal register list.
    pub fn update_rip(&mut self, val: RawPtr) -> Result<()> {
        self.update_register_u64(hv_register_name_HV_X64_REGISTER_RIP, val.into())
    }

    /// Update the value of a specific register in the internally stored
    /// virtual CPU, and store this register update in the pending list
    /// of registers
    ///
    /// This function will apply only the value of the given register on the
    /// internally stored virtual CPU, but no others in the pending list.
    pub fn update_register_u64(&mut self, name: hv_register_name, val: u64) -> Result<()> {
        self.registers
            .insert(name, hv_register_value { reg64: val });
        let reg = hv_register_assoc {
            name,
            value: hv_register_value { reg64: val },
            ..Default::default()
        };
        self.vcpu_fd.set_reg(&[reg]).map_err(|e| anyhow!(e))
    }

    fn run_vcpu(&self) -> Result<hv_message> {
        let hv_message: hv_message = Default::default();
        self.vcpu_fd.run(hv_message).map_err(|e| anyhow!(e))
    }

    fn handle_io_port_intercept(
        &mut self,
        msg: hv_message,
        outb_handle_fn: OutBHandlerRc,
    ) -> Result<()> {
        let io_message = msg.to_ioport_info()?;
        let port_number = io_message.port_number;
        let rax = io_message.rax;
        let rip = io_message.header.rip;
        let instruction_length = io_message.header.instruction_length() as u64;

        outb_handle_fn.call(port_number, rax);

        self.update_rip(RawPtr::from(rip + instruction_length))
    }

    fn handle_unmapped_gpa(
        &self,
        msg: hv_message,
        mem_access_fn: MemAccessHandlerRc,
    ) -> Result<()> {
        mem_access_fn.call();
        let msg_type = msg.header.message_type;
        bail!("Unexpected HyperV exit_reason = {:?}", msg_type)
    }
}

impl Hypervisor for HypervLinuxDriver {
    fn initialise(
        &mut self,
        peb_addr: RawPtr,
        seed: u64,
        page_size: u32,
        outb_hdl: OutBHandlerRc,
        mem_access_hdl: MemAccessHandlerRc,
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
        self.execute_until_halt(outb_hdl, mem_access_hdl)
    }

    fn execute_until_halt(
        &mut self,
        outb_handle_fn: OutBHandlerRc,
        mem_access_fn: MemAccessHandlerRc,
    ) -> Result<()> {
        const HALT_MESSAGE: hv_message_type = hv_message_type_HVMSG_X64_HALT;
        const IO_PORT_INTERCEPT_MESSAGE: hv_message_type =
            hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT;
        const UNMAPPED_GPA_MESSAGE: hv_message_type = hv_message_type_HVMSG_UNMAPPED_GPA;
        loop {
            let run_res = self.run_vcpu()?;
            let hdl_res: Result<()> = match run_res.header.message_type {
                // on a HLT, we're done
                HALT_MESSAGE => return Ok(()),
                // on an IO port intercept, we have to handle a message
                // from the guest.
                IO_PORT_INTERCEPT_MESSAGE => {
                    self.handle_io_port_intercept(run_res, outb_handle_fn.clone())
                }
                // on an unmapped GPA, we have to handle a memory access
                // from the guest
                UNMAPPED_GPA_MESSAGE => self.handle_unmapped_gpa(run_res, mem_access_fn.clone()),
                other => bail!("unknown Hyper-V run message type {:?}", other),
            };
            if let Err(e) = hdl_res {
                bail!(e)
            }
        }
    }

    fn dispatch_call_from_host(
        &mut self,
        dispatch_func_addr: RawPtr,
        outb_handle_fn: OutBHandlerRc,
        mem_access_fn: MemAccessHandlerRc,
    ) -> Result<()> {
        self.update_rip(dispatch_func_addr)?;
        self.execute_until_halt(outb_handle_fn, mem_access_fn)
    }

    fn reset_rsp(&mut self, rsp: u64) -> Result<()> {
        self.update_register_u64(hv_register_name_HV_X64_REGISTER_RSP, rsp)
    }
}

impl Drop for HypervLinuxDriver {
    fn drop(&mut self) {
        match self.vm_fd.unmap_user_memory(self.mem_region) {
            Ok(_) => (),
            Err(e) => {
                // TODO (logging): log this instead of a raw println
                println!("Failed to unmap user memory in HyperVOnLinux ({:?})", e)
            }
        }
    }
}

#[cfg(test)]
pub mod test_cfg {
    use once_cell::sync::Lazy;
    use serde::Deserialize;

    pub static TEST_CONFIG: Lazy<TestConfig> = Lazy::new(|| match envy::from_env::<TestConfig>() {
        Ok(config) => config,
        Err(err) => panic!("error parsing config from env: {}", err),
    });
    pub static SHOULD_RUN_TEST: Lazy<bool> = Lazy::new(is_hyperv_present);

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
    pub struct TestConfig {
        #[serde(default = "hyperv_should_be_present_default")]
        // Set env var HYPERV_SHOULD_BE_PRESENT to require hyperv to be present for the tests.
        pub hyperv_should_be_present: bool,
        #[serde(default = "hyperv_should_have_stable_api_default")]
        // Set env var HYPERV_SHOULD_HAVE_STABLE_API to require a stable api for the tests.
        pub hyperv_should_have_stable_api: bool,
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
pub mod tests {
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
            bail!(
                "code load offset ({}) > memory size ({})",
                u64::from(load_offset),
                mem_size
            )
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
        super::HypervLinuxDriver::new(&addrs).unwrap();
    }

    #[test]
    fn run_vcpu() {
        should_run_hyperv_linux_test!();
        const ACTUAL_MEM_SIZE: usize = 0x4000;
        const REGION_MEM_SIZE: u64 = 0x1000;

        let gm = shared_mem_with_code(CODE.as_slice(), ACTUAL_MEM_SIZE, Offset::zero()).unwrap();
        let addrs = HypervisorAddrs::for_shared_mem(&gm, REGION_MEM_SIZE, 0x1000, 0x1).unwrap();
        let mut driver = HypervLinuxDriver::new(&addrs).unwrap();
        driver.add_basic_registers(&addrs).unwrap();
        driver.apply_registers().unwrap();

        {
            // first instruction should be an IO port intercept

            let run_result = driver.run_vcpu().unwrap();
            let message_type = run_result.header.message_type;
            assert_eq!(hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT, message_type);

            let io_message = run_result.to_ioport_info().unwrap();
            assert!(io_message.rax == b'4' as u64);
            assert!(io_message.port_number == 0x3f8);

            driver
                .update_rip(RawPtr::from(
                    io_message.header.rip + io_message.header.instruction_length() as u64,
                ))
                .unwrap();
        }

        {
            // next, another IO port intercept

            let run_result = driver.run_vcpu().unwrap();
            let message_type = run_result.header.message_type;
            let io_message = run_result.to_ioport_info().unwrap();
            assert_eq!(message_type, hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT);
            assert!(io_message.rax == b'\0' as u64);
            assert!(io_message.port_number == 0x3f8);

            driver
                .update_rip(RawPtr::from(
                    io_message.header.rip + io_message.header.instruction_length() as u64,
                ))
                .unwrap();
        }
        {
            // finally, a halt

            let run_result = driver.run_vcpu().unwrap();
            let message_type = run_result.header.message_type;
            assert_eq!(message_type, hv_message_type_HVMSG_X64_HALT);
        }
    }

    #[test]
    fn new_advanced_config() {
        should_run_hyperv_linux_test!();
        const ACTUAL_MEM_SIZE: usize = 0x4000;
        const REGION_MEM_SIZE: usize = 0x1000;
        let gm = shared_mem_with_code(CODE.as_slice(), ACTUAL_MEM_SIZE, Offset::zero()).unwrap();
        let addrs =
            HypervisorAddrs::for_shared_mem(&gm, REGION_MEM_SIZE as u64, 0x1000, 0x1).unwrap();
        let mut driver = HypervLinuxDriver::new(&addrs).unwrap();
        driver.add_advanced_registers(&addrs, 1, 2).unwrap();
    }
}
