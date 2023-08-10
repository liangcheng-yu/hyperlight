use super::hyperv_windows::WhvRegisterNameWrapper;
use anyhow::Result;
use core::ffi::c_void;
use std::collections::HashMap;
use tracing::instrument;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Hypervisor::*;

// We need to pass in a primitive array of register names/values
// to WHvSetVirtualProcessorRegisters and rust needs to know array size
// at compile time. There is an assert in set_virtual_process_registers
// to ensure we never try and set more registers than this constant
const REGISTER_COUNT: usize = 16;

/// Interop calls for Windows Hypervisor Platform APIs
///
/// Documentation can be found at:
/// - https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/hypervisor-platform
/// - https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Hypervisor/index.html

pub(crate) fn is_hypervisor_present() -> Result<bool> {
    let mut capability: WHV_CAPABILITY = Default::default();
    let written_size: Option<*mut u32> = None;

    unsafe {
        WHvGetCapability(
            WHvCapabilityCodeHypervisorPresent,
            &mut capability as *mut _ as *mut c_void,
            std::mem::size_of::<WHV_CAPABILITY>() as u32,
            written_size,
        )?;
        Ok(capability.HypervisorPresent.as_bool())
    }
}

#[derive(Debug)]
pub(super) struct VMPartition(WHV_PARTITION_HANDLE);

impl VMPartition {
    #[instrument(err(), name = "VMPartition::new")]
    pub(super) fn new(proc_count: u32) -> Result<Self> {
        let hdl = unsafe { WHvCreatePartition() }?;
        Self::set_processor_count(&hdl, proc_count)?;
        unsafe { WHvSetupPartition(hdl) }?;
        Ok(Self(hdl))
    }

    #[instrument(err(), name = "VMPartition::set_processor_count")]
    fn set_processor_count(
        partition_handle: &WHV_PARTITION_HANDLE,
        processor_count: u32,
    ) -> Result<()> {
        unsafe {
            WHvSetPartitionProperty(
                *partition_handle,
                WHvPartitionPropertyCodeProcessorCount,
                &processor_count as *const u32 as *const c_void,
                std::mem::size_of_val(&processor_count) as u32,
            )?;
        }

        Ok(())
    }

    #[instrument(err(), name = "VMPartition::map_gpa_range")]
    pub(super) fn map_gpa_range(
        &mut self,
        process_handle: &HANDLE,
        source_address: *const c_void,
        guest_address: u64,
        size: usize,
        flags: WHV_MAP_GPA_RANGE_FLAGS,
    ) -> Result<()> {
        unsafe {
            WHvMapGpaRange2(
                self.0,
                *process_handle,
                source_address,
                guest_address,
                size.try_into().unwrap(),
                flags,
            )?;
        }

        Ok(())
    }
}

impl Drop for VMPartition {
    fn drop(&mut self) {
        unsafe { WHvDeletePartition(self.0) }.unwrap();
    }
}

#[derive(Debug)]
pub(super) struct VMProcessor(VMPartition);
impl VMProcessor {
    #[instrument(err(), name = "VMProcessor::new")]
    pub(super) fn new(part: VMPartition) -> Result<Self> {
        unsafe { WHvCreateVirtualProcessor(part.0, 0, 0) }?;
        Ok(Self(part))
    }

    fn get_partition_hdl(&self) -> WHV_PARTITION_HANDLE {
        let part = &self.0;
        part.0
    }

    pub(super) fn get_registers(
        &self,
        register_names: &Vec<WHV_REGISTER_NAME>,
    ) -> Result<HashMap<WhvRegisterNameWrapper, WHV_REGISTER_VALUE>> {
        let partition_handle = self.get_partition_hdl();
        let register_count = register_names.len();
        assert!(register_count <= REGISTER_COUNT);
        let mut register_values: [WHV_REGISTER_VALUE; REGISTER_COUNT] = Default::default();

        unsafe {
            WHvGetVirtualProcessorRegisters(
                partition_handle,
                0,
                register_names.as_ptr(),
                register_count as u32,
                register_values.as_mut_ptr(),
            )?;
        }

        let mut registers: HashMap<WhvRegisterNameWrapper, WHV_REGISTER_VALUE> = HashMap::new();

        for i in 0..register_count {
            registers.insert(
                WhvRegisterNameWrapper(register_names[i]),
                register_values[i],
            );
        }

        Ok(registers)
    }

    pub(super) fn set_registers(
        &mut self,
        registers: &HashMap<WhvRegisterNameWrapper, WHV_REGISTER_VALUE>,
    ) -> Result<()> {
        let partition_handle = self.get_partition_hdl();
        let register_count = registers.len();
        assert!(register_count <= REGISTER_COUNT);
        let mut register_names: Vec<WHV_REGISTER_NAME> = vec![];
        let mut register_values: Vec<WHV_REGISTER_VALUE> = vec![];

        for (key, value) in registers.iter() {
            register_names.push(key.0);
            register_values.push(*value);
        }

        unsafe {
            WHvSetVirtualProcessorRegisters(
                partition_handle,
                0,
                register_names.as_ptr(),
                register_count as u32,
                register_values.as_ptr(),
            )?;
        }

        Ok(())
    }

    pub(super) fn run(&mut self) -> Result<WHV_RUN_VP_EXIT_CONTEXT> {
        let partition_handle = self.get_partition_hdl();
        let mut exit_context: WHV_RUN_VP_EXIT_CONTEXT = Default::default();

        unsafe {
            WHvRunVirtualProcessor(
                partition_handle,
                0,
                &mut exit_context as *mut _ as *mut c_void,
                std::mem::size_of::<WHV_RUN_VP_EXIT_CONTEXT>() as u32,
            )?;
        }

        Ok(exit_context)
    }
}

impl Drop for VMProcessor {
    fn drop(&mut self) {
        let part_hdl = self.get_partition_hdl();
        unsafe { WHvDeleteVirtualProcessor(part_hdl, 0) }.unwrap()
    }
}
