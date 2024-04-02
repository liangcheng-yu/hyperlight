use super::hyperv_windows::WhvRegisterNameWrapper;
use crate::Result;
use core::ffi::c_void;
use hyperlight_flatbuffers::mem::{PAGE_SIZE, PAGE_SIZE_USIZE};
use std::collections::HashMap;
use tracing::{instrument, Span};
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
#[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
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
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn new(proc_count: u32) -> Result<Self> {
        let hdl = unsafe { WHvCreatePartition() }?;
        Self::set_processor_count(&hdl, proc_count)?;
        unsafe { WHvSetupPartition(hdl) }?;
        Ok(Self(hdl))
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
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

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn map_gpa_range(
        &mut self,
        process_handle: &HANDLE,
        source_address: *const c_void,
        guest_address: u64,
        stack_guard_offset: u64,
        size: usize,
    ) -> Result<()> {
        unsafe {
            // everything up to but not including guard page
            WHvMapGpaRange2(
                self.0,
                *process_handle,
                source_address,
                guest_address,
                stack_guard_offset,
                WHvMapGpaRangeFlagRead | WHvMapGpaRangeFlagWrite | WHvMapGpaRangeFlagExecute,
            )?;

            // guard page
            WHvMapGpaRange2(
                self.0,
                *process_handle,
                (source_address as *const u8).add(usize::try_from(stack_guard_offset)?)
                    as *const c_void,
                guest_address + stack_guard_offset,
                PAGE_SIZE,
                WHvMapGpaRangeFlagRead,
            )?;

            // everything after guard page
            WHvMapGpaRange2(
                self.0,
                *process_handle,
                (source_address as *const u8)
                    .add(usize::try_from(stack_guard_offset)? + PAGE_SIZE_USIZE)
                    as *const c_void,
                guest_address + stack_guard_offset + PAGE_SIZE,
                u64::try_from(size)? - PAGE_SIZE - stack_guard_offset,
                WHvMapGpaRangeFlagRead | WHvMapGpaRangeFlagWrite,
            )?;
        }

        Ok(())
    }
}

impl Drop for VMPartition {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn drop(&mut self) {
        unsafe { WHvDeletePartition(self.0) }.unwrap();
    }
}

#[derive(Debug)]
pub(super) struct VMProcessor(VMPartition);
impl VMProcessor {
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn new(part: VMPartition) -> Result<Self> {
        unsafe { WHvCreateVirtualProcessor(part.0, 0, 0) }?;
        Ok(Self(part))
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_partition_hdl(&self) -> WHV_PARTITION_HANDLE {
        let part = &self.0;
        part.0
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_registers(
        &self,
        register_names: &Vec<WHV_REGISTER_NAME>,
    ) -> Result<HashMap<WhvRegisterNameWrapper, WHV_REGISTER_VALUE>> {
        let partition_handle = self.get_partition_hdl();
        let register_count = register_names.len();
        assert!(register_count <= REGISTER_COUNT);

        // The creation of the array that follows is done this way rather than just using
        // let mut register_values: [WHV_REGISTER_VALUE; REGISTER_COUNT] = Default::default();
        // as in release builds the compiler is optimising the allocation away and then
        // when we call WHvGetVirtualProcessorRegisters we get a access violation
        // as the memory where the register values are supposed to be written is not allocated
        // See https://github.com/deislabs/hyperlight/actions/runs/6907729617/job/18796236309 for a test run where this happened
        // This issue appeared when the fix in https://github.com/deislabs/hyperlight/pull/1014
        // was introduced. It has the happy effect of only allocating enough memory for the number of
        // registers we actually want to read rather than the maximum number of registers
        let mut register_values: Vec<WHV_REGISTER_VALUE> = vec![];
        for _ in 0..register_count {
            let reg_value = unsafe { std::mem::zeroed::<WHV_REGISTER_VALUE>() };
            register_values.push(reg_value);
        }

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

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
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

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
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
    #[instrument(parent = Span::current(), level= "Trace")]
    fn drop(&mut self) {
        let part_hdl = self.get_partition_hdl();
        unsafe { WHvDeleteVirtualProcessor(part_hdl, 0) }.unwrap()
    }
}
