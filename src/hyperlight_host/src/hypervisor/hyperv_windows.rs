use core::ffi::c_void;
use std::any::Any;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::string::String;
use std::sync::Arc;

use cfg_if::cfg_if;
use crossbeam::atomic::AtomicCell;
use crossbeam_channel::{Receiver, Sender};
use hyperlight_common::mem::PAGE_SIZE_USIZE;
use tracing::{instrument, Span};
use windows::Win32::System::Hypervisor::{
    WHvX64RegisterCr0, WHvX64RegisterCr3, WHvX64RegisterCr4, WHvX64RegisterCs, WHvX64RegisterEfer,
    WHV_MEMORY_ACCESS_TYPE, WHV_PARTITION_HANDLE, WHV_REGISTER_VALUE, WHV_RUN_VP_EXIT_CONTEXT,
    WHV_RUN_VP_EXIT_REASON, WHV_X64_SEGMENT_REGISTER, WHV_X64_SEGMENT_REGISTER_0,
};

use super::fpu::{FP_TAG_WORD_DEFAULT, MXCSR_DEFAULT};
use super::handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper};
use super::surrogate_process::SurrogateProcess;
use super::surrogate_process_manager::*;
use super::windows_hypervisor_platform::{VMPartition, VMProcessor};
use super::wrappers::WHvFPURegisters;
use super::{
    windows_hypervisor_platform as whp, HyperlightExit, Hypervisor, VirtualCPU, CR0_AM, CR0_ET,
    CR0_MP, CR0_NE, CR0_PE, CR0_PG, CR0_WP, CR4_OSFXSR, CR4_OSXMMEXCPT, CR4_PAE, EFER_LMA,
    EFER_LME, EFER_NX, EFER_SCE,
};
use crate::hypervisor::fpu::FP_CONTROL_WORD_DEFAULT;
use crate::hypervisor::hypervisor_handler::{
    HandlerMsg, HasCommunicationChannels, HypervisorHandlerAction,
};
use crate::hypervisor::wrappers::WHvGeneralRegisters;
use crate::mem::memory_region::{MemoryRegion, MemoryRegionFlags};
use crate::mem::ptr::{GuestPtr, RawPtr};
use crate::mem::shared_mem::PtrCVoidMut;
use crate::HyperlightError::{NoHypervisorFound, WindowsErrorHResult};
use crate::{debug, log_then_return, new_error, Result};

/// A Hypervisor driver for HyperV-on-Windows.
pub(crate) struct HypervWindowsDriver {
    size: usize, // this is the size of the memory region, excluding the 2 surrounding guard pages
    processor: VMProcessor,
    surrogate_process: SurrogateProcess,
    source_address: PtrCVoidMut, // this points into the first guard page
    entrypoint: u64,
    orig_rsp: GuestPtr,
    mem_regions: Vec<MemoryRegion>,
    hypervisor_handler_action_transmitter:
        Option<crossbeam_channel::Sender<HypervisorHandlerAction>>,
    hypervisor_handler_action_receiver:
        Option<crossbeam_channel::Receiver<HypervisorHandlerAction>>,
    handler_message_receiver: Option<crossbeam_channel::Receiver<HandlerMsg>>,
    handler_message_transmitter: Option<crossbeam_channel::Sender<HandlerMsg>>,
    cancel_run_requested: Arc<AtomicCell<bool>>,
    join_handle: Option<std::thread::JoinHandle<Result<()>>>,
    // ^^^ a Hypervisor's operations are executed on a Hypervisor Handler thread (i.e.,
    // separate from the main host thread). This is a handle to the Hypervisor Handler thread.
}

impl HypervWindowsDriver {
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub(crate) fn new(
        mem_regions: Vec<MemoryRegion>,
        raw_size: usize,
        raw_source_address: *mut c_void,
        pml4_address: u64,
        entrypoint: u64,
        rsp: u64,
    ) -> Result<Self> {
        if !whp::is_hypervisor_present() {
            log_then_return!(NoHypervisorFound());
        }

        // create and setup hypervisor partition
        let mut partition = VMPartition::new(1)?;

        // get a surrogate process with preallocated memory of size SharedMemory::raw_mem_size()
        // with guard pages setup
        let surrogate_process = {
            let mgr = get_surrogate_process_manager()?;
            mgr.get_surrogate_process(raw_size, raw_source_address)
        }?;

        partition.map_gpa_range(&mem_regions, &surrogate_process.process_handle)?;

        let mut proc = VMProcessor::new(partition)?;
        Self::setup_initial_sregs(&mut proc, pml4_address)?;

        // subtract 2 pages for the guard pages, since when we copy memory to and from surrogate process,
        // we don't want to copy the guard pages themselves (that would cause access violation)
        let mem_size = raw_size - 2 * PAGE_SIZE_USIZE;
        Ok(Self {
            size: mem_size,
            processor: proc,
            surrogate_process,
            source_address: PtrCVoidMut::from(raw_source_address),
            entrypoint,
            orig_rsp: GuestPtr::try_from(RawPtr::from(rsp))?,
            mem_regions,
            hypervisor_handler_action_transmitter: None,
            hypervisor_handler_action_receiver: None,
            handler_message_receiver: None,
            handler_message_transmitter: None,
            cancel_run_requested: Arc::new(AtomicCell::new(false)),
            join_handle: None,
        })
    }

    fn setup_initial_sregs(proc: &mut VMProcessor, pml4_addr: u64) -> Result<()> {
        proc.set_registers(&[
            (WHvX64RegisterCr3, WHV_REGISTER_VALUE { Reg64: pml4_addr }),
            (
                WHvX64RegisterCr4,
                WHV_REGISTER_VALUE {
                    Reg64: CR4_PAE | CR4_OSFXSR | CR4_OSXMMEXCPT,
                },
            ),
            (
                WHvX64RegisterCr0,
                WHV_REGISTER_VALUE {
                    Reg64: CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_AM | CR0_PG | CR0_WP,
                },
            ),
            (
                WHvX64RegisterEfer,
                WHV_REGISTER_VALUE {
                    Reg64: EFER_LME | EFER_LMA | EFER_SCE | EFER_NX,
                },
            ),
            (
                WHvX64RegisterCs,
                WHV_REGISTER_VALUE {
                    Segment: WHV_X64_SEGMENT_REGISTER {
                        Anonymous: WHV_X64_SEGMENT_REGISTER_0 {
                            Attributes: 0b1011 | 1 << 4 | 1 << 7 | 1 << 13, // Type (11: Execute/Read, accessed) | L (64-bit mode) | P (present) | S (code segment)
                        },
                        ..Default::default() // zero out the rest
                    },
                },
            ),
        ])?;
        Ok(())
    }

    #[inline]
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn get_exit_details(&self, exit_reason: WHV_RUN_VP_EXIT_REASON) -> Result<String> {
        let mut error = String::new();
        error.push_str(&format!(
            "Did not receive a halt from Hypervisor as expected - Received {exit_reason:?}!\n"
        ));
        error.push_str(&format!("Registers: \n{:#?}", self.processor.get_regs()?));
        Ok(error)
    }

    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    pub(super) fn get_partition_hdl(&self) -> WHV_PARTITION_HANDLE {
        self.processor.get_partition_hdl()
    }
}

impl Debug for HypervWindowsDriver {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut fs = f.debug_struct("HyperV Driver");

        fs.field("Size", &self.size)
            .field("Source Address", &self.source_address)
            .field("Entrypoint", &self.entrypoint)
            .field("Original RSP", &self.orig_rsp);

        for region in &self.mem_regions {
            fs.field("Memory Region", &region);
        }

        // Get the registers

        let regs = self.processor.get_regs();

        if let Ok(regs) = regs {
            {
                fs.field("Registers", &regs);
            }
        }

        // Get the special registers

        let special_regs = self.processor.get_sregs();
        if let Ok(special_regs) = special_regs {
            fs.field("CR0", unsafe { &special_regs.cr0.Reg64 });
            fs.field("CR2", unsafe { &special_regs.cr2.Reg64 });
            fs.field("CR3", unsafe { &special_regs.cr3.Reg64 });
            fs.field("CR4", unsafe { &special_regs.cr4.Reg64 });
            fs.field("CR8", unsafe { &special_regs.cr8.Reg64 });
            fs.field("EFER", unsafe { &special_regs.efer.Reg64 });
            fs.field("APIC_BASE", unsafe { &special_regs.apic_base.Reg64 });

            // Segment registers
            fs.field(
                "CS",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Selector: {:?}, Attributes: {:?} }}",
                    unsafe { &special_regs.cs.Segment.Base },
                    unsafe { &special_regs.cs.Segment.Limit },
                    unsafe { &special_regs.cs.Segment.Selector },
                    unsafe { &special_regs.cs.Segment.Anonymous.Attributes }
                ),
            );
            fs.field(
                "DS",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Selector: {:?}, Attributes: {:?} }}",
                    unsafe { &special_regs.ds.Segment.Base },
                    unsafe { &special_regs.ds.Segment.Limit },
                    unsafe { &special_regs.ds.Segment.Selector },
                    unsafe { &special_regs.ds.Segment.Anonymous.Attributes }
                ),
            );
            fs.field(
                "ES",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Selector: {:?}, Attributes: {:?} }}",
                    unsafe { &special_regs.es.Segment.Base },
                    unsafe { &special_regs.es.Segment.Limit },
                    unsafe { &special_regs.es.Segment.Selector },
                    unsafe { &special_regs.es.Segment.Anonymous.Attributes }
                ),
            );
            fs.field(
                "FS",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Selector: {:?}, Attributes: {:?} }}",
                    unsafe { &special_regs.fs.Segment.Base },
                    unsafe { &special_regs.fs.Segment.Limit },
                    unsafe { &special_regs.fs.Segment.Selector },
                    unsafe { &special_regs.fs.Segment.Anonymous.Attributes }
                ),
            );
            fs.field(
                "GS",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Selector: {:?}, Attributes: {:?} }}",
                    unsafe { &special_regs.gs.Segment.Base },
                    unsafe { &special_regs.gs.Segment.Limit },
                    unsafe { &special_regs.gs.Segment.Selector },
                    unsafe { &special_regs.gs.Segment.Anonymous.Attributes }
                ),
            );
            fs.field(
                "SS",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Selector: {:?}, Attributes: {:?} }}",
                    unsafe { &special_regs.ss.Segment.Base },
                    unsafe { &special_regs.ss.Segment.Limit },
                    unsafe { &special_regs.ss.Segment.Selector },
                    unsafe { &special_regs.ss.Segment.Anonymous.Attributes }
                ),
            );
            fs.field(
                "TR",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Selector: {:?}, Attributes: {:?} }}",
                    unsafe { &special_regs.tr.Segment.Base },
                    unsafe { &special_regs.tr.Segment.Limit },
                    unsafe { &special_regs.tr.Segment.Selector },
                    unsafe { &special_regs.tr.Segment.Anonymous.Attributes }
                ),
            );
            fs.field(
                "LDTR",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Selector: {:?}, Attributes: {:?} }}",
                    unsafe { &special_regs.ldtr.Segment.Base },
                    unsafe { &special_regs.ldtr.Segment.Limit },
                    unsafe { &special_regs.ldtr.Segment.Selector },
                    unsafe { &special_regs.ldtr.Segment.Anonymous.Attributes }
                ),
            );
            fs.field(
                "GDTR",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Pad: {:?} }}",
                    unsafe { &special_regs.gdtr.Table.Base },
                    unsafe { &special_regs.gdtr.Table.Limit },
                    unsafe { &special_regs.gdtr.Table.Pad }
                ),
            );
            fs.field(
                "IDTR",
                &format_args!(
                    "{{ Base: {:?}, Limit: {:?}, Pad: {:?} }}",
                    unsafe { &special_regs.idtr.Table.Base },
                    unsafe { &special_regs.idtr.Table.Limit },
                    unsafe { &special_regs.idtr.Table.Pad }
                ),
            );
        };

        fs.finish()
    }
}

impl Hypervisor for HypervWindowsDriver {
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn initialise(
        &mut self,
        peb_address: RawPtr,
        seed: u64,
        page_size: u32,
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
    ) -> Result<()> {
        let regs = WHvGeneralRegisters {
            rip: self.entrypoint,
            rsp: self.orig_rsp.absolute()?,

            // function args
            rcx: peb_address.into(),
            rdx: seed,
            r8: page_size.into(),
            r9: self.get_max_log_level().into(),
            rflags: 1 << 1, // eflags bit index 1 is reserved and always needs to be 1

            ..Default::default()
        };
        self.processor.set_general_purpose_registers(&regs)?;

        VirtualCPU::run(self.as_mut_hypervisor(), outb_hdl, mem_access_hdl)?;

        // reset RSP to what it was before initialise
        self.processor
            .set_general_purpose_registers(&WHvGeneralRegisters {
                rsp: self.orig_rsp.absolute()?,
                ..Default::default()
            })?;
        Ok(())
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn dispatch_call_from_host(
        &mut self,
        dispatch_func_addr: RawPtr,
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
    ) -> Result<()> {
        // Reset general purpose registers except RSP, then set RIP
        let rsp_before = self.processor.get_regs()?.rsp;
        let regs = WHvGeneralRegisters {
            rip: dispatch_func_addr.into(),
            rsp: rsp_before,
            rflags: 1 << 1, // eflags bit index 1 is reserved and always needs to be 1
            ..Default::default()
        };
        self.processor.set_general_purpose_registers(&regs)?;

        // reset fpu state
        self.processor.set_fpu(&WHvFPURegisters {
            fp_control_word: FP_CONTROL_WORD_DEFAULT,
            fp_tag_word: FP_TAG_WORD_DEFAULT,
            mxcsr: MXCSR_DEFAULT,
            ..Default::default() // zero out the rest
        })?;

        VirtualCPU::run(self.as_mut_hypervisor(), outb_hdl, mem_access_hdl)?;

        // reset RSP to what it was before function call
        self.processor
            .set_general_purpose_registers(&WHvGeneralRegisters {
                rsp: rsp_before,
                ..Default::default()
            })?;
        Ok(())
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
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
            .map_err(|e| new_error!("error locking {}", e))?
            .call(port, u64::from_le_bytes(payload))?;

        let mut regs = self.processor.get_regs()?;
        regs.rip = rip + instruction_length;
        self.processor.set_general_purpose_registers(&regs)
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    fn run(&mut self) -> Result<super::HyperlightExit> {
        let bytes_written: Option<*mut usize> = None;
        let bytes_read: Option<*mut usize> = None;

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
                self.surrogate_process
                    .allocated_address
                    .as_ptr()
                    .add(PAGE_SIZE_USIZE),
                self.source_address.as_ptr().add(PAGE_SIZE_USIZE),
                self.size,
                bytes_written,
            )
            .as_bool()
            {
                let hresult = windows::Win32::Foundation::GetLastError();
                log_then_return!(WindowsErrorHResult(hresult.to_hresult()));
            }
        }

        // - call WHvRunVirtualProcessor
        let exit_context: WHV_RUN_VP_EXIT_CONTEXT = self.processor.run()?;

        // - call read-process memory
        unsafe {
            if !windows::Win32::System::Diagnostics::Debug::ReadProcessMemory(
                self.surrogate_process.process_handle,
                self.surrogate_process
                    .allocated_address
                    .as_ptr()
                    .add(PAGE_SIZE_USIZE),
                self.source_address.as_mut_ptr().add(PAGE_SIZE_USIZE),
                self.size,
                bytes_read,
            )
            .as_bool()
            {
                let hresult = windows::Win32::Foundation::GetLastError();
                log_then_return!(WindowsErrorHResult(hresult.to_hresult()));
            }
        }

        let result = match exit_context.ExitReason {
            // WHvRunVpExitReasonX64IoPortAccess
            WHV_RUN_VP_EXIT_REASON(2i32) => {
                // size of current instruction is in lower byte of _bitfield
                // see https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/funcs/whvexitcontextdatatypes)
                let instruction_length = exit_context.VpContext._bitfield & 0xF;
                unsafe {
                    cfg_if! {
                        if #[cfg(all(feature = "print_debug", debug_assertions))] {
                            println!(
                                "HyperV IO Details :\n Port: {:#x} \n {:#?}",
                                exit_context.Anonymous.IoPortAccess.PortNumber, &self
                            );
                        } else {
                            debug!(
                                "HyperV IO Details :\n Port: {:#x} \n {:#?}",
                                exit_context.Anonymous.IoPortAccess.PortNumber, &self
                            );
                        }
                    }
                    HyperlightExit::IoOut(
                        exit_context.Anonymous.IoPortAccess.PortNumber,
                        exit_context
                            .Anonymous
                            .IoPortAccess
                            .Rax
                            .to_le_bytes()
                            .to_vec(),
                        exit_context.VpContext.Rip,
                        instruction_length as u64,
                    )
                }
            }
            // HvRunVpExitReasonX64Halt
            WHV_RUN_VP_EXIT_REASON(8i32) => {
                debug!("HyperV Halt Details :\n {:#?}", &self);
                HyperlightExit::Halt()
            }
            // WHvRunVpExitReasonMemoryAccess
            WHV_RUN_VP_EXIT_REASON(1i32) => {
                let gpa = unsafe { exit_context.Anonymous.MemoryAccess.Gpa };
                let access_info = unsafe {
                    WHV_MEMORY_ACCESS_TYPE(
                        // 2 first bits are the access type, see https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/funcs/memoryaccess#syntax
                        (exit_context.Anonymous.MemoryAccess.AccessInfo.AsUINT32 & 0b11) as i32,
                    )
                };
                let access_info = MemoryRegionFlags::try_from(access_info)?;
                debug!(
                    "HyperV Memory Access Details :\n GPA: {:#?}\n Access Info :{:#?}\n {:#?} ",
                    gpa, access_info, &self
                );
                #[cfg(all(debug_assertions, feature = "dump_on_crash"))]
                {
                    if let Err(e) = unsafe {
                        self.write_dump_file(
                            self.mem_regions.clone(),
                            self.source_address.as_ptr().add(PAGE_SIZE_USIZE) as *const u8,
                            self.size,
                        )
                    } {
                        println!("Error dumping memory: {}", e);
                    }
                }

                match self.get_memory_access_violation(gpa as usize, &self.mem_regions, access_info)
                {
                    Some(access_info) => access_info,
                    None => HyperlightExit::Mmio(gpa),
                }
            }
            //  WHvRunVpExitReasonCanceled
            //  Execution was cancelled by the host.
            //  This will happen when guest code runs for too long
            WHV_RUN_VP_EXIT_REASON(8193i32) => {
                debug!("HyperV Cancelled Details :\n {:#?}", &self);
                HyperlightExit::Cancelled()
            }
            WHV_RUN_VP_EXIT_REASON(_) => {
                debug!(
                    "HyperV Unexpected Exit Details :#nReason {:#?}\n {:#?}",
                    exit_context.ExitReason, &self
                );
                #[cfg(all(debug_assertions, feature = "dump_on_crash"))]
                {
                    if let Err(e) = unsafe {
                        self.write_dump_file(
                            self.mem_regions.clone(),
                            self.source_address.as_ptr().add(PAGE_SIZE_USIZE) as *const u8,
                            self.size,
                        )
                    } {
                        println!("Error dumping memory: {}", e);
                    }
                }
                match self.get_exit_details(exit_context.ExitReason) {
                    Ok(error) => HyperlightExit::Unknown(error),
                    Err(e) => HyperlightExit::Unknown(format!("Error getting exit details: {}", e)),
                }
            }
        };

        Ok(result)
    }

    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[instrument(skip_all, parent = Span::current(), level = "Trace")]
    fn as_mut_hypervisor(&mut self) -> &mut dyn Hypervisor {
        self as &mut dyn Hypervisor
    }

    fn set_handler_join_handle(&mut self, handle: std::thread::JoinHandle<Result<()>>) {
        self.join_handle = Some(handle);
    }

    fn get_mut_handler_join_handle(&mut self) -> &mut Option<std::thread::JoinHandle<Result<()>>> {
        &mut self.join_handle
    }

    fn set_termination_status(&mut self, value: bool) {
        log::debug!("Setting termination status to {}", value);
        self.cancel_run_requested.store(value);
    }

    fn get_termination_status(&self) -> Arc<AtomicCell<bool>> {
        self.cancel_run_requested.clone()
    }
}

impl HasCommunicationChannels for HypervWindowsDriver {
    fn get_to_handler_tx(&self) -> Sender<HypervisorHandlerAction> {
        self.hypervisor_handler_action_transmitter.clone().unwrap()
    }
    fn set_to_handler_tx(&mut self, tx: Sender<HypervisorHandlerAction>) {
        self.hypervisor_handler_action_transmitter = Some(tx);
    }
    fn drop_to_handler_tx(&mut self) {
        self.hypervisor_handler_action_transmitter = None;
    }

    fn get_from_handler_rx(&self) -> Receiver<HandlerMsg> {
        self.handler_message_receiver.clone().unwrap()
    }
    fn set_from_handler_rx(&mut self, rx: Receiver<HandlerMsg>) {
        self.handler_message_receiver = Some(rx);
    }

    fn get_from_handler_tx(&self) -> Sender<HandlerMsg> {
        self.handler_message_transmitter.clone().unwrap()
    }
    fn set_from_handler_tx(&mut self, tx: Sender<HandlerMsg>) {
        self.handler_message_transmitter = Some(tx);
    }

    fn get_to_handler_rx(&self) -> Receiver<HypervisorHandlerAction> {
        self.hypervisor_handler_action_receiver.clone().unwrap()
    }
    fn set_to_handler_rx(&mut self, rx: Receiver<HypervisorHandlerAction>) {
        self.hypervisor_handler_action_receiver = Some(rx);
    }
}

#[cfg(test)]
pub mod tests {
    use std::sync::{Arc, Mutex};

    use serial_test::serial;

    use super::HypervWindowsDriver;
    use crate::hypervisor::handlers::{MemAccessHandler, OutBHandler};
    use crate::hypervisor::tests::test_initialise;
    use crate::mem::layout::SandboxMemoryLayout;
    use crate::mem::ptr::GuestPtr;
    use crate::mem::ptr_offset::Offset;
    use crate::Result;

    extern "C" fn outb_fn(_port: u16, _payload: u64) {}

    extern "C" fn mem_access_fn() {}

    #[test]
    #[serial]
    fn test_init() {
        let outb_handler = {
            let func: Box<dyn FnMut(u16, u64) -> Result<()> + Send> =
                Box::new(|_, _| -> Result<()> { Ok(()) });
            Arc::new(Mutex::new(OutBHandler::from(func)))
        };
        let mem_access_handler = {
            let func: Box<dyn FnMut() -> Result<()> + Send> = Box::new(|| -> Result<()> { Ok(()) });
            Arc::new(Mutex::new(MemAccessHandler::from(func)))
        };
        test_initialise(
            outb_handler,
            mem_access_handler,
            |mgr, rsp_ptr, pml4_ptr| {
                let host_addr = mgr.shared_mem.raw_ptr();
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
                    mgr.layout.get_memory_regions(&mgr.shared_mem)?,
                    mgr.shared_mem.raw_mem_size(),
                    host_addr,
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
