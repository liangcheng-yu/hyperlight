#[cfg(target_os = "windows")]
use super::loaded_lib::LoadedLib;
use super::{
    layout::SandboxMemoryLayout,
    pe::{headers::PEHeaders, pe_info::PEInfo},
    ptr::{GuestPtr, RawPtr},
    ptr_offset::Offset,
    shared_mem::SharedMemory,
    shared_mem_snapshot::SharedMemorySnapshot,
};
use crate::{
    error::HyperlightError::{
        ExceptionDataLengthIncorrect, ExceptionMessageTooBig, JsonConversionFailure,
        NoMemorySnapshot, UTF8SliceConversionFailure,
    },
    log_then_return,
};
use crate::{error::HyperlightHostError, sandbox::SandboxConfiguration};
use crate::{new_error, Result};
use core::mem::size_of;

use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::{validate_guest_function_call_buffer, validate_host_function_call_buffer},
    guest_error::ErrorCode,
};

use hyperlight_flatbuffers::flatbuffer_wrappers::{
    function_call::FunctionCall, function_types::ReturnValue, guest_error::GuestError,
    guest_log_data::GuestLogData, host_function_details::HostFunctionDetails,
};
use serde_json::from_str;
use std::{cmp::Ordering, str::from_utf8};
use tracing::{instrument, Span};

/// Whether or not the 64-bit page directory entry (PDE) record is
/// present.
///
/// See the following links explaining a PDE in various levels of detail:
///
/// * Very basic description: https://stackoverflow.com/a/26945892
/// * More in-depth descriptions: https://wiki.osdev.org/Paging
const PDE64_PRESENT: u64 = 1;
/// Read/write permissions flag for the 64-bit PDE
const PDE64_RW: u64 = 1 << 1;
/// The user/supervisor bit for the 64-bit PDE
const PDE64_USER: u64 = 1 << 2;
/// The page size for the 64-bit PDE
const PDE64_PS: u64 = 1 << 7;
/// The size of stack guard cookies
pub(crate) const STACK_COOKIE_LEN: usize = 16;

/// A struct that is responsible for laying out and managing the memory
/// for a given `Sandbox`.
#[derive(Clone)]
//TODO:(#1029) Once we have a full C API visibility can be pub(crate)
pub struct SandboxMemoryManager {
    /// Whether or not to run a sandbox in-process
    run_from_process_memory: bool,
    mem_snapshot: Option<SharedMemorySnapshot>,
    /// Shared memory for the Sandbox
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    pub shared_mem: SharedMemory,
    pub(crate) layout: SandboxMemoryLayout,
    /// Pointer to where to load memory from
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    pub load_addr: RawPtr,
    /// Offset for the execution entrypoint from `load_addr`
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    pub entrypoint_offset: Offset,
    /// This field must be present, even though it's not read,
    /// so that its underlying resources are properly dropped at
    /// the right time.
    #[cfg(target_os = "windows")]
    _lib: Option<LoadedLib>,
}

impl SandboxMemoryManager {
    /// Create a new `SandboxMemoryManager` with the given parameters
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn new(
        layout: SandboxMemoryLayout,
        shared_mem: SharedMemory,
        run_from_process_memory: bool,
        load_addr: RawPtr,
        entrypoint_offset: Offset,
        #[cfg(target_os = "windows")] lib: Option<LoadedLib>,
    ) -> Self {
        Self {
            run_from_process_memory,
            mem_snapshot: None,
            layout,
            shared_mem,
            load_addr,
            entrypoint_offset,
            #[cfg(target_os = "windows")]
            _lib: lib,
        }
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn is_in_process(&self) -> bool {
        self.run_from_process_memory
    }

    /// Get `SharedMemory` in `self` as a mutable reference
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_shared_mem_mut(&mut self) -> &mut SharedMemory {
        &mut self.shared_mem
    }

    /// Set the stack guard to `cookie` using `layout` to calculate
    /// its location and `shared_mem` to write it.
    ///
    /// Currently, this method could be an associated function but is
    /// still a method because I (arschles) want to make this `struct` hold a
    /// reference to a `SandboxMemoryLayout` and `SharedMemory`,
    /// remove the `layout` and `shared_mem` parameters, and use
    /// the `&self` to access them instead.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn set_stack_guard(&mut self, cookie: &[u8; STACK_COOKIE_LEN]) -> Result<()> {
        let stack_offset = self.layout.get_top_of_stack_offset();
        self.shared_mem.copy_from_slice(cookie, stack_offset)
    }

    /// Set up the hypervisor partition in the given `SharedMemory` parameter
    /// `shared_mem`, with the given memory size `mem_size`
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_up_hypervisor_partition(&mut self, mem_size: u64) -> Result<u64> {
        // Add 0x200000 because that's the start of mapped memory
        // For MSVC, move rsp down by 0x28.  This gives the called 'main'
        // function the appearance that rsp was was 16 byte aligned before
        // the 'call' that calls main (note we don't really have a return value
        // on the stack but some assembly instructions are expecting rsp have
        // started 0x8 bytes off of 16 byte alignment when 'main' is invoked.
        // We do 0x28 instead of 0x8 because MSVC can expect that there are
        // 0x20 bytes of space to write to by the called function.
        // I am not sure if this happens with the 'main' method, but we do this
        // just in case.
        //
        // NOTE: We do this also for GCC freestanding binaries because we
        // specify __attribute__((ms_abi)) on the start method
        let rsp = mem_size + SandboxMemoryLayout::BASE_ADDRESS as u64 - 0x28;

        // Create pagetable
        self.shared_mem.write_u64(
            Offset::try_from(SandboxMemoryLayout::PML4_OFFSET)?,
            PDE64_PRESENT | PDE64_RW | PDE64_USER | SandboxMemoryLayout::PDPT_GUEST_ADDRESS as u64,
        )?;
        self.shared_mem.write_u64(
            Offset::try_from(SandboxMemoryLayout::PDPT_OFFSET)?,
            PDE64_PRESENT | PDE64_RW | PDE64_USER | SandboxMemoryLayout::PD_GUEST_ADDRESS as u64,
        )?;

        // do not map first 2 megs
        for i in 0..512 {
            let offset = Offset::try_from(SandboxMemoryLayout::PD_OFFSET + (i * 8))?;
            // map each VA to physical memory 2 megs lower
            let val_to_write: u64 =
                (i << 21) as u64 + (PDE64_PRESENT | PDE64_RW | PDE64_USER | PDE64_PS);
            self.shared_mem.write_u64(offset, val_to_write)?;
        }
        Ok(rsp)
    }

    /// Check the stack guard of the memory in `shared_mem`, using
    /// `layout` to calculate its location.
    ///
    /// Return `true`
    /// if `shared_mem` could be accessed properly and the guard
    /// matches `cookie`. If it could be accessed properly and the
    /// guard doesn't match `cookie`, return `false`. Otherwise, return
    /// a descriptive error.
    ///
    /// This method could be an associated function instead. See
    /// documentation at the bottom `set_stack_guard` for description
    /// of why it isn't.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn check_stack_guard(&self, cookie: [u8; STACK_COOKIE_LEN]) -> Result<bool> {
        let offset = self.layout.get_top_of_stack_offset();
        let mut test_cookie = vec![b'\0'; cookie.len()];
        self.shared_mem
            .copy_to_slice(test_cookie.as_mut_slice(), offset)?;

        let cmp_res = cookie.iter().cmp(test_cookie.iter());
        Ok(cmp_res == Ordering::Equal)
    }

    /// Get the process environment block (PEB) address assuming `start_addr`
    /// is the address of the start of memory, using the given
    /// `SandboxMemoryLayout` to calculate the address.
    ///
    /// For more details on PEBs, please see the following link:
    ///
    /// https://en.wikipedia.org/wiki/Process_Environment_Block
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn get_peb_address(&self, start_addr: u64) -> Result<u64> {
        match self.run_from_process_memory {
            true => {
                let updated_offset = self.layout.get_in_process_peb_offset() + start_addr;
                Ok(u64::from(updated_offset))
            }
            false => u64::try_from(self.layout.peb_address).map_err(|_| {
                new_error!(
                    "get_peb_address: failed to convert peb_address ({}) to u64",
                    self.layout.peb_address
                )
            }),
        }
    }

    /// Create a new memory snapshot of the given `SharedMemory` and
    /// store it internally. Return an `Ok(())` if the snapshot
    /// operation succeeded, and an `Err` otherwise.
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn snapshot_state(&mut self) -> Result<()> {
        let snap = &mut self.mem_snapshot;
        if let Some(snapshot) = snap {
            snapshot.replace_snapshot()
        } else {
            let new_snapshot = SharedMemorySnapshot::new(self.shared_mem.clone())?;
            self.mem_snapshot = Some(new_snapshot);
            Ok(())
        }
    }

    /// Restore memory from the pre-existing snapshot and return
    /// `Ok(())`. Return an `Err` if there was no pre-existing
    /// snapshot, or there was but there was an error restoring.
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn restore_state(&mut self) -> Result<()> {
        let snap = &mut self.mem_snapshot;
        if let Some(snapshot) = snap {
            snapshot.restore_from_snapshot()
        } else {
            log_then_return!(NoMemorySnapshot);
        }
    }

    /// Get the return value of an executable that ran, or an `Err`
    /// if no such return value was present.
    //TODO:(#1029) Once we have a full C API this can be removed.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn get_return_value(&self) -> Result<i32> {
        let offset = self.layout.output_data_buffer_offset;
        self.shared_mem.read_i32(offset)
    }

    /// Sets `addr` to the correct offset in the memory referenced by
    /// `shared_mem` to indicate the address of the outb pointer
    ///
    /// TODO: this function is only in C#. Remove it once we have a full Rust
    /// Sandbox
    //TODO: Once we have a full C API this can be removed.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_outb_address(&mut self, addr: u64) -> Result<()> {
        let offset = self.layout.get_outb_pointer_offset();
        self.shared_mem.write_u64(offset, addr)
    }

    /// Sets `addr` to the correct offset in the memory referenced by
    /// `shared_mem` to indicate the address of the outb pointer and context
    /// for calling outb function
    #[cfg(target_os = "windows")]
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn set_outb_address_and_context(&mut self, addr: u64, context: u64) -> Result<()> {
        let offset = self.layout.get_outb_pointer_offset();
        self.shared_mem.write_u64(offset, addr)?;
        let offset = self.layout.get_outb_context_offset();
        self.shared_mem.write_u64(offset, context)
    }

    /// Get the address of the dispatch function in memory
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn get_pointer_to_dispatch_function(&self) -> Result<u64> {
        let guest_dispatch_function_ptr = self
            .shared_mem
            .read_u64(self.layout.get_dispatch_function_pointer_offset())?;

        // This pointer is written by the guest library but is accessible to
        // the guest engine so we should bounds check it before we return it.
        //
        // When executing with in-hypervisor mode, there is no danger from
        // the guest manipulating this memory location because the only
        // addresses that are valid are in its own address space.
        //
        // When executing in-process, maniulating this pointer could cause the
        // host to execute arbitary functions.
        let guest_ptr = GuestPtr::try_from(RawPtr::from(guest_dispatch_function_ptr))?;
        guest_ptr.absolute()
    }

    /// Get the length of the host exception
    //TODO:(#1029) Once we have a full C API visibility can be private
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn get_host_error_length(&self) -> Result<i32> {
        let offset = self.layout.get_host_exception_offset();
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        self.shared_mem.read_i32(offset)
    }

    /// Get a bool indicating if there is a host error
    //TODO:(#1029) Once we have a full C API visibility can be private
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn has_host_error(&self) -> Result<bool> {
        let offset = self.layout.get_host_exception_offset();
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        let len = self.shared_mem.read_i32(offset)?;
        Ok(len != 0)
    }

    /// Get the error data that was written by the Hyperlight Host
    /// Returns a `Result` containing 'Unit' or an error.Error
    /// Writes the exception data to the buffer at `exception_data_ptr`.
    ///
    /// TODO: after the C API wrapper for this function goes away,
    /// have this function return a Vec<u8> instead of requiring
    /// the user pass in a slice of the same length as returned by
    /// self.get_host_error_length()
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn get_host_error_data(&self, exception_data_slc: &mut [u8]) -> Result<()> {
        let offset = self.layout.get_host_exception_offset();
        let len = self.get_host_error_length()?;

        let exception_data_slc_len = exception_data_slc.len();
        if exception_data_slc_len != len as usize {
            log_then_return!(ExceptionDataLengthIncorrect(len, exception_data_slc_len));
        }
        // The host exception field is expected to contain a 32-bit length followed by the exception data.
        self.shared_mem
            .copy_to_slice(exception_data_slc, offset + size_of::<i32>())?;
        Ok(())
    }

    /// Look for a `HyperlightError` generated by the host, and return
    /// an `Ok(Some(the_error))` if we succeeded in looking for one, and
    /// it was found. Return `Ok(None)` if we succeeded in looking for
    /// one and it wasn't found. Return an `Err` if we did not succeed
    /// in looking for one.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_host_error(&self) -> Result<Option<HyperlightHostError>> {
        if self.has_host_error()? {
            let host_err_len = {
                let len_i32 = self.get_host_error_length()?;
                usize::try_from(len_i32)
            }?;
            // create a Vec<u8> of length host_err_len.
            // it's important we set the length, rather than just
            // the capacity, because self.get_host_error_data ensures
            // the length of the vec matches the return value of
            // self.get_host_error_length()
            let mut host_err_data: Vec<u8> = vec![0; host_err_len];
            self.get_host_error_data(&mut host_err_data)?;
            let host_err_json = from_utf8(&host_err_data).map_err(UTF8SliceConversionFailure)?;
            let host_err: HyperlightHostError =
                from_str(host_err_json).map_err(JsonConversionFailure)?;
            Ok(Some(host_err))
        } else {
            Ok(None)
        }
    }

    /// This function writes an error to guest memory and is intended to be
    /// used when the host's outb handler code raises an error.
    //TODO:(#1029) Once we have a full C API visibility can be private
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn write_outb_error(
        &mut self,
        guest_error_msg: &Vec<u8>,
        host_exception_data: &Vec<u8>,
    ) -> Result<()> {
        let message = String::from_utf8(guest_error_msg.to_owned())?;
        let ge = GuestError::new(ErrorCode::OutbError, message);

        let guest_error_buffer: Vec<u8> = (&ge)
            .try_into()
            .map_err(|_| new_error!("write_outb_error: failed to convert GuestError to Vec<u8>"))?;

        let err_buffer_size_offset = self.layout.get_guest_error_buffer_size_offset();
        let max_err_buffer_size = self.shared_mem.read_u64(err_buffer_size_offset)?;

        if guest_error_buffer.len() as u64 > max_err_buffer_size {
            log_then_return!("The guest error message is too large to fit in the shared memory");
        }
        self.shared_mem.copy_from_slice(
            guest_error_buffer.as_slice(),
            self.layout.guest_error_buffer_offset,
        )?;

        let host_exception_offset = self.layout.get_host_exception_offset();
        let host_exception_size_offset = self.layout.get_host_exception_size_offset();
        let max_host_exception_size = {
            let size_u64 = self.shared_mem.read_u64(host_exception_size_offset)?;
            usize::try_from(size_u64)
        }?;

        // First four bytes of host exception are length

        if host_exception_data.len() > max_host_exception_size - size_of::<i32>() {
            log_then_return!(ExceptionMessageTooBig(
                host_exception_data.len(),
                max_host_exception_size - size_of::<i32>()
            ));
        }

        self.shared_mem
            .write_i32(host_exception_offset, host_exception_data.len() as i32)?;
        self.shared_mem.copy_from_slice(
            host_exception_data,
            host_exception_offset + size_of::<i32>(),
        )?;

        Ok(())
    }

    /// Get the guest error data
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn get_guest_error(&self) -> Result<GuestError> {
        // get memory buffer max size
        let err_buffer_size_offset = self.layout.get_guest_error_buffer_size_offset();
        let max_err_buffer_size = self.shared_mem.read_u64(err_buffer_size_offset)?;

        // get guest error from layout and shared mem
        let mut guest_error_buffer = vec![b'0'; usize::try_from(max_err_buffer_size)?];
        let err_msg_offset = self.layout.guest_error_buffer_offset;
        self.shared_mem
            .copy_to_slice(guest_error_buffer.as_mut_slice(), err_msg_offset)?;
        GuestError::try_from(guest_error_buffer.as_slice()).map_err(|e| {
            new_error!(
                "get_guest_error: failed to convert buffer to GuestError: {}",
                e
            )
        })
    }

    /// Load the binary represented by `pe_info` into memory, ensuring
    /// all necessary relocations are made prior to completing the load
    /// operation, then create a new `SharedMemory` to store the new PE
    /// file and a `SandboxMemoryLayout` to describe the layout of that
    /// new `SharedMemory`.
    ///
    /// Returns the following:
    ///
    /// - The newly-created `SharedMemory`
    /// - The `SandboxMemoryLayout` describing that `SharedMemory`
    /// - The offset to the entrypoint. This value means something different
    /// depending on whether we're using in-process mode or not:
    ///     - If we're using in-process mode, this value will be into
    ///     host memory
    ///     - If we're not running with in-memory mode, this value will be
    ///     into guest memory
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn load_guest_binary_into_memory(
        cfg: SandboxConfiguration,
        pe_info: &mut PEInfo,
        run_from_process_memory: bool,
    ) -> Result<Self> {
        let (layout, mut shared_mem, load_addr, entrypoint_offset) =
            load_guest_binary_common(cfg, pe_info, pe_info.get_payload_len(), |shared_mem| {
                let addr_usize = if run_from_process_memory {
                    // if we're running in-process, load_addr is the absolute
                    // address to the start of shared memory, plus the offset to
                    // code
                    shared_mem.base_addr() + SandboxMemoryLayout::CODE_OFFSET
                } else {
                    // otherwise, we're running in a VM, so load_addr
                    // is the base address in a VM plus the code
                    // offset
                    SandboxMemoryLayout::GUEST_CODE_ADDRESS
                };
                RawPtr::try_from(addr_usize)
            })?;

        let relocation_patches = pe_info
            .get_exe_relocation_patches(pe_info.get_payload(), load_addr.clone().try_into()?)?;

        {
            // Apply relocations to the PE file (if necessary), then copy
            // the PE file into shared memory
            PEInfo::apply_relocation_patches(pe_info.get_payload_mut(), relocation_patches)?;
            let code_offset = Offset::try_from(SandboxMemoryLayout::CODE_OFFSET)?;
            shared_mem.copy_from_slice(pe_info.get_payload(), code_offset)
        }?;

        Ok(Self::new(
            layout,
            shared_mem,
            run_from_process_memory,
            load_addr,
            entrypoint_offset,
            #[cfg(target_os = "windows")]
            None,
        ))
    }

    /// Similar to load_guest_binary_into_memory, except only works on Windows
    /// and uses the
    /// [`LoadLibraryA`](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibrarya)
    /// function.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn load_guest_binary_using_load_library(
        cfg: SandboxConfiguration,
        guest_bin_path: &str,
        pe_info: &mut PEInfo,
        run_from_process_memory: bool,
    ) -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            let lib = LoadedLib::try_from(guest_bin_path)?;
            let (layout, shared_mem, load_addr, entrypoint_offset) =
                load_guest_binary_common(cfg, pe_info, 0, |_| lib.base_addr())?;
            Ok(Self::new(
                layout,
                shared_mem,
                run_from_process_memory,
                load_addr,
                entrypoint_offset,
                Some(lib),
            ))
        }
        #[cfg(target_os = "linux")]
        {
            // these assignments to nothing prevent clippy from complaining,
            // on non-windows systems, that this function's parameters
            // are unused
            let _ = cfg;
            let _ = guest_bin_path;
            let _ = pe_info;
            let _ = run_from_process_memory;
            log_then_return!("load_guest_binary_using_load_library is only available on Windows");
        }
    }

    /// Writes host function details to memory
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn write_buffer_host_function_details(&mut self, buffer: &[u8]) -> Result<()> {
        let host_function_details = HostFunctionDetails::try_from(buffer).map_err(|e| {
            new_error!(
                "write_buffer_host_function_details: failed to convert buffer to HostFunctionDetails: {}",
                e
            )
        })?;

        let host_function_call_buffer: Vec<u8> = (&host_function_details).try_into().map_err(|_| {
            new_error!(
                "write_buffer_host_function_details: failed to convert HostFunctionDetails to Vec<u8>"
            )
        })?;

        let buffer_size = {
            let size_u64 = self
                .shared_mem
                .read_u64(self.layout.get_host_function_definitions_size_offset())?;
            usize::try_from(size_u64)
        }?;

        if host_function_call_buffer.len() > buffer_size {
            log_then_return!(
                "Host Function Details buffer is too big for the host_function_definitions buffer"
            );
        }

        self.shared_mem.copy_from_slice(
            host_function_call_buffer.as_slice(),
            self.layout.host_function_definitions_offset,
        )?;
        Ok(())
    }

    /// Writes a guest function call to memory
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn write_guest_function_call(&mut self, buffer: &[u8]) -> Result<()> {
        let layout = self.layout;

        let buffer_size = {
            let size_u64 = self
                .shared_mem
                .read_u64(layout.get_input_data_size_offset())?;
            usize::try_from(size_u64)
        }?;

        if buffer.len() > buffer_size {
            return Err(new_error!(
                "Guest function call buffer {} is too big for the input data buffer {}",
                buffer.len(),
                buffer_size
            ));
        }

        validate_guest_function_call_buffer(buffer).map_err(|e| {
            new_error!(
                "Guest function call buffer validation failed: {}",
                e.to_string()
            )
        })?;

        self.shared_mem
            .copy_from_slice(buffer, layout.input_data_buffer_offset)?;

        Ok(())
    }

    /// Writes a host function call to memory
    //TODO:(#1029) Once we have a full C API visibility can be private
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn write_host_function_call(&mut self, buffer: &[u8]) -> Result<()> {
        let layout = self.layout;

        let buffer_size = {
            let size_u64 = self
                .shared_mem
                .read_u64(layout.get_output_data_size_offset())?;
            usize::try_from(size_u64)
        }?;

        if buffer.len() > buffer_size {
            return Err(new_error!(
                "Host function call buffer {} is too big for the output data buffer {}",
                buffer.len(),
                buffer_size
            ));
        }

        validate_host_function_call_buffer(buffer)
            .map_err(|e| new_error!("Invalid host function call buffer: {}", e.to_string()))?;
        self.shared_mem
            .copy_from_slice(buffer, layout.host_function_definitions_offset)?;

        Ok(())
    }

    /// Writes a function call result to memory
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn write_response_from_host_method_call(&mut self, res: &ReturnValue) -> Result<()> {
        let input_data_offset = self.layout.input_data_buffer_offset;
        let function_call_ret_val_buffer = Vec::<u8>::try_from(res).map_err(|_| {
            new_error!(
                "write_response_from_host_method_call: failed to convert ReturnValue to Vec<u8>"
            )
        })?;
        self.shared_mem
            .copy_from_slice(function_call_ret_val_buffer.as_slice(), input_data_offset)
    }

    /// Reads a host function call from memory
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn get_host_function_call(&self) -> Result<FunctionCall> {
        let layout = self.layout;

        // Get the size of the flatbuffer buffer from memory
        let fb_buffer_size = {
            let size_i32 = self.shared_mem.read_i32(layout.output_data_buffer_offset)? + 4;
            usize::try_from(size_i32)
        }?;

        let mut function_call_buffer = vec![0; fb_buffer_size];
        self.shared_mem
            .copy_to_slice(&mut function_call_buffer, layout.output_data_buffer_offset)?;
        #[cfg(debug_assertions)]
        validate_host_function_call_buffer(&function_call_buffer)
            .map_err(|e| new_error!("Invalid host function call buffer: {}", e.to_string()))?;

        FunctionCall::try_from(function_call_buffer.as_slice()).map_err(|e| {
            new_error!(
                "get_host_function_call: failed to convert buffer to FunctionCall: {}",
                e
            )
        })
    }

    /// Reads a guest function call from memory
    //TODO: Why is this unused?
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    #[allow(unused)]
    fn get_guest_function_call(&self) -> Result<FunctionCall> {
        let layout = self.layout;

        // read guest function call from memory
        let fb_buffer_size = {
            let size_i32 = self.shared_mem.read_i32(layout.input_data_buffer_offset)? + 4;
            usize::try_from(size_i32)
        }?;

        let mut function_call_buffer = vec![0; fb_buffer_size];
        self.shared_mem
            .copy_to_slice(&mut function_call_buffer, layout.input_data_buffer_offset)?;

        #[cfg(debug_assertions)]
        validate_guest_function_call_buffer(&function_call_buffer).map_err(|e| {
            new_error!(
                "get_guest_function_call: failed to validate guest function call buffer: {}",
                e
            )
        })?;

        FunctionCall::try_from(function_call_buffer.as_slice()).map_err(|e| {
            new_error!(
                "get_guest_function_call: failed to convert buffer to FunctionCall: {}",
                e
            )
        })
    }

    /// Reads a function call result from memory
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn get_function_call_result(&self) -> Result<ReturnValue> {
        let fb_buffer_size = {
            let size_i32 = self
                .shared_mem
                .read_i32(self.layout.output_data_buffer_offset)?
                + 4;
            // ^^^ flatbuffer byte arrays are prefixed by 4 bytes
            // indicating its size, so, to get the actual size, we need
            // to add 4.
            usize::try_from(size_i32)
        }?;

        let mut function_call_result_buffer = vec![0; fb_buffer_size];

        self.shared_mem.copy_to_slice(
            &mut function_call_result_buffer,
            self.layout.output_data_buffer_offset,
        )?;
        ReturnValue::try_from(function_call_result_buffer.as_slice()).map_err(|e| {
            new_error!(
                "get_function_call_result: failed to convert buffer to ReturnValue: {}",
                e
            )
        })
    }

    /// Read guest log data from the `SharedMemory` contained within `self`
    //TODO:(#1029) Once we have a full C API visibility can be pub(crate)
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn read_guest_log_data(&self) -> Result<GuestLogData> {
        let offset = self.layout.get_output_data_offset();
        // there's a u32 at the beginning of the GuestLogData
        // with the size
        let size = self.shared_mem.read_u32(offset)?;
        // read size + 32 bits from shared memory, starting at
        // layout.get_output_data_offset
        let mut vec_out = {
            let len_usize = usize::try_from(size)? + size_of::<u32>();
            vec![0; len_usize]
        };
        self.shared_mem
            .copy_to_slice(vec_out.as_mut_slice(), offset)?;
        GuestLogData::try_from(vec_out.as_slice()).map_err(|e| {
            new_error!(
                "read_guest_log_data: failed to convert buffer to GuestLogData: {}",
                e
            )
        })
    }
}

/// Common setup functionality for the
/// `load_guest_binary_{into_memory, using_load_library}` functions
///
/// Returns the newly created `SandboxMemoryLayout`, newly created
/// `SharedMemory`, load address as calculated by `load_addr_fn`,
/// and calculated entrypoint offset, in order.
#[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
fn load_guest_binary_common<F>(
    cfg: SandboxConfiguration,
    pe_info: &PEInfo,
    code_size: usize,
    load_addr_fn: F,
) -> Result<(SandboxMemoryLayout, SharedMemory, RawPtr, Offset)>
where
    F: FnOnce(&SharedMemory) -> Result<RawPtr>,
{
    let layout = SandboxMemoryLayout::new(
        cfg,
        code_size,
        usize::try_from(cfg.get_stack_size(pe_info))?,
        usize::try_from(cfg.get_heap_size(pe_info))?,
    )?;
    let mut shared_mem = SharedMemory::new(layout.get_memory_size()?)?;

    let load_addr: RawPtr = load_addr_fn(&shared_mem)?;

    let entrypoint_offset = Offset::from({
        // we have to create this intermediate variable to ensure
        // we have an _immutable_ reference to a `PEInfo`, which
        // is what the PEHeaders::from expects
        let pe_info_immut: &PEInfo = pe_info;
        let pe_headers = PEHeaders::from(pe_info_immut);
        pe_headers.entrypoint_offset
    });

    let offset = layout.get_code_pointer_offset();

    {
        // write the code pointer to shared memory
        let load_addr_u64: u64 = load_addr.clone().try_into()?;
        shared_mem.write_u64(offset, load_addr_u64)?;
    }
    Ok((layout, shared_mem, load_addr, entrypoint_offset))
}

#[cfg(test)]
mod tests {
    use super::SandboxMemoryManager;
    use crate::{
        error::HyperlightHostError,
        mem::{
            layout::SandboxMemoryLayout, pe::pe_info::PEInfo, ptr::RawPtr, ptr_offset::Offset,
            shared_mem::SharedMemory,
        },
        sandbox::SandboxConfiguration,
        testing::bytes_for_path,
    };
    use hyperlight_testing::{callback_guest_as_pathbuf, rust_guest_as_pathbuf};
    use serde_json::to_string;
    #[cfg(target_os = "windows")]
    use serial_test::serial;

    #[test]
    fn load_guest_binary_common() {
        let guests = vec![
            rust_guest_as_pathbuf("simpleguest"),
            callback_guest_as_pathbuf(),
        ];
        for guest in guests {
            let guest_bytes = bytes_for_path(guest).unwrap();
            let pe_info = PEInfo::new(guest_bytes.as_slice()).unwrap();
            let stack_size_override = 0x3000;
            let heap_size_override = 0x10000;
            let mut cfg = SandboxConfiguration::default();
            cfg.set_stack_size(stack_size_override);
            cfg.set_heap_size(heap_size_override);
            let (layout, shared_mem, _, _) =
                super::load_guest_binary_common(cfg, &pe_info, 100, |_| Ok(RawPtr::from(100)))
                    .unwrap();
            assert_eq!(
                stack_size_override,
                u64::try_from(layout.stack_size).unwrap()
            );
            assert_eq!(heap_size_override, u64::try_from(layout.heap_size).unwrap());
            assert_eq!(layout.get_memory_size().unwrap(), shared_mem.mem_size());
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[serial]
    fn load_guest_binary_using_load_library() {
        use crate::mem::mgr::SandboxMemoryManager;
        use hyperlight_testing::{rust_guest_as_pathbuf, simple_guest_as_string};

        let cfg = SandboxConfiguration::default();
        let guest_path = rust_guest_as_pathbuf("simpleguest");
        let guest_bytes = bytes_for_path(guest_path).unwrap();
        let mut pe_info = PEInfo::new(guest_bytes.as_slice()).unwrap();
        let _ = SandboxMemoryManager::load_guest_binary_using_load_library(
            cfg,
            simple_guest_as_string().unwrap().as_str(),
            &mut pe_info,
            true,
        )
        .unwrap();
    }

    /// Don't write a host error, try to read it back, and verify we
    /// successfully do the read but get no error back
    #[test]
    fn get_host_error_none() {
        let cfg = SandboxConfiguration::default();
        let layout = SandboxMemoryLayout::new(cfg, 0x10000, 0x10000, 0x10000).unwrap();
        let mut shared_mem = SharedMemory::new(layout.get_memory_size().unwrap()).unwrap();
        let mem_size = shared_mem.mem_size();
        layout
            .write(&mut shared_mem, SandboxMemoryLayout::BASE_ADDRESS, mem_size)
            .unwrap();
        let mgr = SandboxMemoryManager::new(
            layout,
            shared_mem,
            false,
            RawPtr::from(0),
            Offset::from(0),
            #[cfg(target_os = "windows")]
            None,
        );
        assert_eq!(None, mgr.get_host_error().unwrap());
    }

    /// write a host error to shared memory, then try to read it back out
    #[test]
    fn round_trip_host_error() {
        let cfg = SandboxConfiguration::default();
        let layout = SandboxMemoryLayout::new(cfg, 0x10000, 0x10000, 0x10000).unwrap();
        let mem_size = layout.get_memory_size().unwrap();
        // write a host error and then try to read it back
        let mut shared_mem = SharedMemory::new(mem_size).unwrap();
        layout
            .write(&mut shared_mem, SandboxMemoryLayout::BASE_ADDRESS, mem_size)
            .unwrap();
        let mut mgr = SandboxMemoryManager::new(
            layout,
            shared_mem.clone(),
            false,
            RawPtr::from(0),
            Offset::from(0),
            #[cfg(target_os = "windows")]
            None,
        );
        let err = HyperlightHostError {
            message: "test message".to_string(),
            source: "rust test".to_string(),
        };
        let err_json_bytes = {
            let str = to_string(&err).unwrap();
            str.into_bytes()
        };
        let err_json_msg = "test error message".to_string().into_bytes();
        mgr.write_outb_error(&err_json_msg, &err_json_bytes)
            .unwrap();

        let host_err_opt = mgr
            .get_host_error()
            .expect("get_host_err should return an Ok");
        assert!(host_err_opt.is_some());
        assert_eq!(err, host_err_opt.unwrap());
    }
}
