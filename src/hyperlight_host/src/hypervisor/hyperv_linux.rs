use super::{
    handlers::{MemAccessHandlerRc, OutBHandlerRc},
    Hypervisor, CR0_AM, CR0_ET, CR0_MP, CR0_NE, CR0_PE, CR0_PG, CR0_WP, CR4_OSFXSR, CR4_OSXMMEXCPT,
    CR4_PAE, EFER_LMA, EFER_LME,
};

use crate::mem::ptr::RawPtr;
use crate::{hypervisor::hypervisor_mem::HypervisorAddrs, mem::ptr::GuestPtr};
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

type RegistersHashMap = HashMap<hv_register_name, hv_register_value>;

/// A Hypervisor driver for HyperV-on-Linux. This hypervisor is often
/// called the Microsoft Hypervisor Platform (MSHV)
pub(crate) struct HypervLinuxDriver {
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
    pub(crate) fn new(
        addrs: &HypervisorAddrs,
        rsp_ptr: GuestPtr,
        pml4_ptr: GuestPtr,
    ) -> Result<Self> {
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
        let mut vcpu_fd = vm_fd.create_vcpu(0).map_err(|e| anyhow!(e))?;
        let mem_region = mshv_user_mem_region {
            size: addrs.mem_size,
            guest_pfn: addrs.guest_pfn,
            userspace_addr: addrs.host_addr,
            flags: HV_MAP_GPA_READABLE | HV_MAP_GPA_WRITABLE | HV_MAP_GPA_EXECUTABLE,
        };

        vm_fd.map_user_memory(mem_region).map_err(|e| anyhow!(e))?;
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
            vcpu.get_reg(std::slice::from_mut(&mut cs_reg))
                .map_err(|e| anyhow!(e))?;
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
    pub(crate) fn apply_registers(&self) -> Result<()> {
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
    pub(crate) fn update_rip(&mut self, val: RawPtr) -> Result<()> {
        self.update_register_u64(hv_register_name_HV_X64_REGISTER_RIP, val.into())
    }

    /// Update the value of a specific register in the internally stored
    /// virtual CPU, and store this register update in the pending list
    /// of registers
    ///
    /// This function will apply only the value of the given register on the
    /// internally stored virtual CPU, but no others in the pending list.
    pub(crate) fn update_register_u64(&mut self, name: hv_register_name, val: u64) -> Result<()> {
        self.registers
            .insert(name, hv_register_value { reg64: val });
        let reg = hv_register_assoc {
            name,
            value: hv_register_value { reg64: val },
            ..Default::default()
        };
        self.vcpu_fd.set_reg(&[reg]).map_err(|e| anyhow!(e))
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
        /// Run the given `vcpu` until the next interrupt and return an `Ok`
        /// with the `hv_message` representing the interrupt.
        ///
        /// Will return an `Ok` if _any_ interrupt was successfully made,
        /// even if the caller considers it a "failure".
        ///
        /// Note: this is defined as a static function (an `fn`) rather than
        /// a closure (an `Fn`, `FnMut`, or `FnOnce`) because we can't close
        /// over `self.vcpu` so we can mutate it below. Defining this as a
        /// `fn` ensures the compiler will prevent us from closing over
        /// anything. If you're modifying this function and need to access
        /// more state inside `execute_until_halt`, pass it as a parameter to
        /// `run_vcpu`.
        fn run_vcpu(vcpu: &VcpuFd) -> Result<hv_message> {
            let hv_message: hv_message = Default::default();
            vcpu.run(hv_message).map_err(|e| anyhow!(e))
        }

        const HALT_MESSAGE: hv_message_type = hv_message_type_HVMSG_X64_HALT;
        const IO_PORT_INTERCEPT_MESSAGE: hv_message_type =
            hv_message_type_HVMSG_X64_IO_PORT_INTERCEPT;
        const UNMAPPED_GPA_MESSAGE: hv_message_type = hv_message_type_HVMSG_UNMAPPED_GPA;
        loop {
            let run_res = run_vcpu(&self.vcpu_fd)?;
            match run_res.header.message_type {
                // we've succeeded if we get a halt, so we can return success
                HALT_MESSAGE => return Ok(()),
                // on an IO port intercept, we have to handle a message sent
                // from the guest.
                IO_PORT_INTERCEPT_MESSAGE => {
                    self.handle_io_port_intercept(run_res, outb_handle_fn.clone())?;
                }
                // "unmapped GPA" indicates a request to a guest physical
                // address (GPA) that is not mapped to valid memory.
                //
                // in this case, we need to call the mem_access_fn callback and
                // then fail
                UNMAPPED_GPA_MESSAGE => {
                    mem_access_fn.call();
                    let msg_type = run_res.header.message_type;
                    bail!("Linux HyperV unmapped GPA. exit_reason = {:?}", msg_type);
                }
                other => bail!("unknown Hyper-V run message type {:?}", other),
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

    fn orig_rsp(&self) -> Result<u64> {
        self.orig_rsp.absolute()
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
        let rsp_ptr = GuestPtr::try_from(Offset::from(0)).unwrap();
        let pml4_ptr = GuestPtr::try_from(Offset::from(0)).unwrap();
        super::HypervLinuxDriver::new(&addrs, rsp_ptr, pml4_ptr).unwrap();
    }
}
