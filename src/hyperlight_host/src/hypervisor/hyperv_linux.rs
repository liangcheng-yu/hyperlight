use crate::capi::hyperv_linux::HV_X64_REGISTER_RCX;

use super::U128;
use anyhow::{bail, Result};
use mshv_bindings::*;
use mshv_ioctls::{Mshv, VcpuFd, VmFd};

/// Determine whether the HyperV for Linux hypervisor API is present
/// and functional. If `require_stable_api` is true, determines only whether a
/// stable API for the Linux HyperV hypervisor is present.
pub fn is_hypervisor_present(require_stable_api: bool) -> Result<bool> {
    let mshv = Mshv::new()?;
    match mshv.check_stable() {
        Ok(stable) => {
            if stable {
                Ok(true)
            } else {
                Ok(!require_stable_api)
            }
        }
        Err(e) => bail!(e),
    }
}
/// The constant to map guest physical addresses as readable
/// in an mshv memory region
pub const HV_MAP_GPA_READABLE: u32 = 1;
/// The constant to map guest physical addresses as writable
/// in an mshv memory region
pub const HV_MAP_GPA_WRITABLE: u32 = 2;
/// The constant to map guest physical addresses as executable
/// in an mshv memory region
pub const HV_MAP_GPA_EXECUTABLE: u32 = 12;
const HV_X64_REGISTER_CR0: u32 = 262144;
const HV_X64_REGISTER_CR3: u32 = 262146;
const HV_X64_REGISTER_CR4: u32 = 262147;
const HV_X64_REGISTER_EFER: u32 = 524289;
const HV_X64_REGISTER_RIP: u32 = 131088;
const HV_X64_REGISTER_RFLAGS: u32 = 131089;
const HV_X64_REGISTER_CS: u32 = 393217;
const HV_X64_REGISTER_RSP: u32 = 131076;
const HV_X64_REGISTER_RDX: u32 = 131074;
const CR4_PAE: u64 = 1 << 5;
const CR4_OSFXSR: u64 = 1 << 9;
const CR4_OSXMMEXCPT: u64 = 1 << 10;
const CR0_PE: u64 = 1;
const CR0_MP: u64 = 1 << 1;
const CR0_ET: u64 = 1 << 4;
const CR0_NE: u64 = 1 << 5;
const CR0_WP: u64 = 1 << 16;
const CR0_AM: u64 = 1 << 18;
const CR0_PG: u64 = 1 << 31;
const EFER_LME: u64 = 1 << 8;
const EFER_LMA: u64 = 1 << 10;

/// A Hypervisor driver for HyperV-on-Linux. This hypervisor is often
/// called the Microsoft Hypervisor Platform (MSHV)
pub struct HypervLinuxDriver {
    _mshv: Mshv,
    vm_fd: VmFd,
    vcpu_fd: VcpuFd,
    mem_region: mshv_user_mem_region,
    registers: Vec<hv_register_assoc>,
}

impl HypervLinuxDriver {
    /// Create a new instance of a HypervLinuxDriver with the given
    /// guest page file number, memory address on the host, and memory size.
    pub fn new(
        require_stable_api: bool,
        entrypoint: u64,
        rsp: u64,
        pml4_addr: u64,
        guest_pfn: u64,
        load_address: u64,
        size: u64,
    ) -> Result<Self> {
        match is_hypervisor_present(require_stable_api) {
            Ok(true) => (),
            Ok(false) => bail!(
                "Hypervisor not present (stable api was {:?})",
                require_stable_api
            ),
            Err(e) => bail!(e),
        }
        let mshv = Mshv::new().map_err(|e| anyhow::anyhow!(e))?;
        let pr = Default::default();
        let vm_fd = mshv
            .create_vm_with_config(&pr)
            .map_err(|e| anyhow::anyhow!(e))?;
        let vcpu_fd = vm_fd.create_vcpu(0).map_err(|e| anyhow::anyhow!(e))?;
        let mem_region = mshv_user_mem_region {
            flags: HV_MAP_GPA_READABLE | HV_MAP_GPA_WRITABLE | HV_MAP_GPA_EXECUTABLE,
            guest_pfn,
            size,
            userspace_addr: load_address as u64,
        };

        vm_fd
            .map_user_memory(mem_region)
            .map_err(|e| anyhow::anyhow!(e))?;

        let mut ret = Self {
            _mshv: mshv,
            vm_fd,
            vcpu_fd,
            mem_region,
            registers: Vec::new(),
        };
        ret.add_register(
            HV_X64_REGISTER_CR3,
            U128 {
                low: pml4_addr,
                high: 0,
            },
        );
        ret.add_register(
            HV_X64_REGISTER_CR4,
            U128 {
                low: CR4_PAE | CR4_OSFXSR | CR4_OSXMMEXCPT,
                high: 0,
            },
        );
        ret.add_register(
            HV_X64_REGISTER_CR0,
            U128 {
                low: CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_WP | CR0_AM | CR0_PG,
                high: 0,
            },
        );
        ret.add_register(
            HV_X64_REGISTER_EFER,
            U128 {
                low: EFER_LME | EFER_LMA,
                high: 0,
            },
        );
        ret.add_register(
            HV_X64_REGISTER_CS,
            U128 {
                low: 0,
                high: 0xa09b0008ffffffff,
            },
        );
        ret.add_register(
            HV_X64_REGISTER_RFLAGS,
            U128 {
                low: 0x0002,
                high: 0,
            },
        );
        ret.add_register(
            HV_X64_REGISTER_RIP,
            U128 {
                low: entrypoint,
                high: 0,
            },
        );
        ret.add_register(HV_X64_REGISTER_RSP, U128 { low: rsp, high: 0 });
        Ok(ret)
    }

    /// Add registers to the internal register list, but do not
    /// set them on the internally stored virtual CPU.
    ///
    /// If you want to set the internal register list on the stored
    /// virtual CPU, call `set_registers`
    pub fn add_registers(&mut self, regs: &[hv_register_assoc]) {
        for reg in regs {
            self.add_register_native(reg)
        }
    }
    fn add_register_native(&mut self, reg: &hv_register_assoc) {
        self.registers.push(*reg)
    }
    fn add_register(&mut self, reg_name: u32, val: U128) {
        let native_reg = hv_register_assoc {
            name: reg_name,
            reserved1: 0,
            reserved2: 0,
            value: hv_register_value::from(val),
        };
        self.add_register_native(&native_reg);
    }

    /// Set the internally stored register list on the internally
    /// stored virtual CPU.
    ///
    /// Call `add_registers` prior to this function to add to the internal
    /// register list.
    pub fn set_registers(&self) -> Result<()> {
        self.vcpu_fd
            .set_reg(self.registers.as_slice())
            .map_err(|e| anyhow::anyhow!(e))
    }

    fn run_vcpu(&self) -> Result<hv_message> {
        let hv_message: hv_message = Default::default();
        self.vcpu_fd.run(hv_message).map_err(|e| anyhow::anyhow!(e))
    }

    /// Initialise the internally stored vCPU with the given PEB address and
    /// random number seed, then run it until a HLT instruction.
    pub fn initialise(&mut self, peb_addr: u64, seed: u64) -> Result<()> {
        self.add_register(HV_X64_REGISTER_RCX, U128::from(peb_addr));
        self.add_register(HV_X64_REGISTER_RDX, U128::from(seed));
        self.set_registers()?;
        self.execute_until_halt()
    }

    /// Run the internally stored vCPU until a HLT instruction.
    pub fn execute_until_halt(&self) -> Result<()> {
        const HALT_MESSAGE: hv_message_type = hv_message_type_HVMSG_X64_HALT;
        const IO_PORT_INTERCEPT_MESSAGE: hv_message_type =
            hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT;
        const UNMAPPED_GPA_MESSAGE: hv_message_type = hv_message_type_HVMSG_UNMAPPED_GPA;
        loop {
            let run_res = self.run_vcpu()?;
            let hdl_res: Result<()> = match run_res.header.message_type {
                // on a HLT, we're done
                HALT_MESSAGE => Ok(()),
                // on an IO port intercept, we have to handle a message
                // from the guest.
                IO_PORT_INTERCEPT_MESSAGE => self.handle_io_port_intercept(run_res),
                // on an unmapped GPA, we have to handle a memory access
                // from the guest
                UNMAPPED_GPA_MESSAGE => self.handle_unmapped_gpa(run_res),
                other => bail!("unknown Hyper-V run message type {:?}", other),
            };
            if let Err(e) = hdl_res {
                bail!(e)
            }
        }
    }

    fn handle_io_port_intercept(&self, _msg: hv_message) -> Result<()> {
        todo!()
    }

    fn handle_unmapped_gpa(&self, _msg: hv_message) -> Result<()> {
        todo!()
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
            "SHOULD_HAVE_STABLE_API is {}",
            TEST_CONFIG.should_have_stable_api
        );
        println!(
            "HYPERV_SHOULD_BE_PRESENT is {}",
            TEST_CONFIG.hyperv_should_be_present
        );
        let is_present =
            super::is_hypervisor_present(TEST_CONFIG.should_have_stable_api).unwrap_or(false);
        if (is_present && !TEST_CONFIG.hyperv_should_be_present)
            || (!is_present && TEST_CONFIG.hyperv_should_be_present)
        {
            panic!(
                "WARNING Hyper-V is present returned  {}, should be present is: {} SHOULD_HAVE_STABLE_API is {}",
                is_present, TEST_CONFIG.hyperv_should_be_present, TEST_CONFIG.should_have_stable_api
            );
        }
        is_present
    }
    fn hyperv_should_be_present_default() -> bool {
        false
    }

    fn should_have_stable_api_default() -> bool {
        false
    }
    #[derive(Deserialize, Debug)]
    pub struct TestConfig {
        #[serde(default = "hyperv_should_be_present_default")]
        // Set env var HYPERV_SHOULD_BE_PRESENT to require hyperv to be present for the tests.
        pub hyperv_should_be_present: bool,
        #[serde(default = "should_have_stable_api_default")]
        // Set env var SHOULD_HAVE_STABLE_API to require a stable api for the tests.
        pub should_have_stable_api: bool,
    }

    #[macro_export]
    macro_rules! should_run_hyperv_linux_test {
        () => {{
            if !(*SHOULD_RUN_TEST) {
                println! {"Not Running Test SHOULD_RUN_TEST is false"}
                return Ok(());
            }
            println! {"Running Test SHOULD_RUN_TEST is true"}
        }};
    }
}
#[cfg(test)]
pub mod tests {
    use super::test_cfg::TEST_CONFIG;
    #[test]
    fn test_is_hypervisor_present() {
        let result = super::is_hypervisor_present(true).unwrap_or(false);
        assert_eq!(
            result,
            TEST_CONFIG.hyperv_should_be_present && TEST_CONFIG.should_have_stable_api
        );
        assert!(!result);
        let result = super::is_hypervisor_present(false).unwrap_or(false);
        assert_eq!(result, TEST_CONFIG.hyperv_should_be_present);
    }
}
