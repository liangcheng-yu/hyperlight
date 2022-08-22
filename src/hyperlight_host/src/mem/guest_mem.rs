use anyhow::{anyhow, bail, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[cfg(target_os = "linux")]
use libc::mmap;
use std::ffi::c_void;
use std::io::Cursor;
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT, PAGE_EXECUTE_READWRITE};

macro_rules! bounds_check {
    ($offset:expr, $size:expr) => {
        if $offset >= $size {
            bail!(
                "offset {} out of bounds (max size: size {})",
                $offset,
                $size,
            );
        }
    };
}
/// GuestMemory is a representation of the guests's
/// physical memory, often referred to as Guest Physical
/// Memory or Guest Physical Addresses (GPA) in Windows
/// Hypervisor Platform
#[derive(Copy, Clone, Debug)]
pub struct GuestMemory {
    ptr: *mut c_void,
    size: usize,
}

impl GuestMemory {
    /// Create a new region of guest memory with the given minimum
    /// size in bytes.
    ///
    /// Return `Err` if guest memory could not be allocated.
    pub fn new(min_size_bytes: usize) -> Result<Self> {
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                // https://docs.rs/libc/latest/libc/fn.mmap.html
                let mmap_addr = unsafe {
                    mmap(
                        null_mut(),
                        min_size_bytes,
                        libc::PROT_READ | libc::PROT_WRITE,
                        libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
                        -1,
                        0,
                    )
                };

                Ok(Self{ptr: mmap_addr, size: min_size_bytes})
            } else {
                let mmap_addr = unsafe {
                    VirtualAlloc(
                        null_mut(),
                        min_size_bytes,
                        MEM_COMMIT,
                        PAGE_EXECUTE_READWRITE,
                    )
                };
                // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Memory/fn.VirtualAlloc.html
                // windows::Win32::System::Memory::VirtualAlloc
                Ok(Self{ptr: mmap_addr, size: min_size_bytes})
            }
        }
    }

    /// Get the base address of guest memory as a `usize`.
    /// This method is equivalent to casting the internal
    /// `*const c_void` pointer to a `usize` by doing
    /// `pointer as usize`.
    ///
    /// # Safety
    ///
    /// This function should not be used to do pointer artithmetic.
    /// Only use it to get the base address of the memory map so you
    /// can do things like calculate offsets, etc...
    pub fn base_addr(&self) -> usize {
        self.ptr as usize
    }

    /// Copy all bytes in `from_bytes` into the guest memory contained
    /// within `self`.
    ///
    /// If `from_bytes` is smaller than the size of the guest memory within
    /// self, this function does not overwrite the remainder. If it is
    /// larger, this function copy only the first N bytes where N = the
    /// size of guest memory
    pub fn copy_into(&mut self, from_bytes: &[u8], address: usize) -> Result<()> {
        bounds_check!(address, self.size);
        bounds_check!(address + from_bytes.len(), self.size);
        unsafe { self.write_slice(from_bytes, from_bytes.len(), address) }
    }

    /// Read an `i64` from guest memory starting at `offset`
    ///
    /// Return `Ok` with the `i64` value starting at `offset`
    /// if the value between `offset` and `offset + <64 bits>`
    /// was successfully decoded to a little-endian `i64`,
    /// and `Err` otherwise.
    pub fn read_i64(&self, offset: u64) -> Result<i64> {
        bounds_check!(offset, self.size as u64);
        let slc = unsafe { self.as_slice() };
        let mut c = Cursor::new(slc);
        c.set_position(offset);
        c.read_i64::<LittleEndian>().map_err(|e| anyhow!(e))
    }

    /// Write val into guest memory at the given offset
    /// from the start of guest memory
    pub fn write_u64(&mut self, offset: usize, val: u64) -> Result<()> {
        bounds_check!(offset, self.size);
        // write the u64 into 8 bytes, so we can std::ptr::write
        // them into guest mem
        let mut writer = vec![];
        writer.write_u64::<LittleEndian>(val)?;
        let slice = unsafe { self.as_mut_slice() };
        for (idx, item) in writer.iter().enumerate() {
            slice[offset + idx] = *item;
        }
        Ok(())
    }

    unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        // inspired by https://docs.rs/mmap-rs/0.3.0/src/mmap_rs/lib.rs.html#309
        std::slice::from_raw_parts_mut(self.ptr as *mut u8, self.size)
    }

    unsafe fn as_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.ptr as *const u8, self.size)
    }

    unsafe fn write_slice(&mut self, slc: &[u8], len: usize, offset: usize) -> Result<()> {
        bounds_check!(offset, self.size);
        let num_bytes = if len > slc.len() { slc.len() } else { len };

        let dst_ptr = {
            let ptr = self.ptr as *mut u8;
            ptr.add(offset)
        };

        std::ptr::copy(slc.as_ptr(), dst_ptr, num_bytes);

        Ok(())
    }

    /// Write `val` to `slc` as little-endian at `offset`.
    ///
    /// If `Ok` is returned, `slc` will have been modified
    /// in-place. Otherwise, no modifications will have been
    /// made.
    pub fn write_usize(&mut self, offset: usize, val: u64) -> Result<()> {
        bounds_check!(offset, self.size);
        let slc = unsafe { self.as_mut_slice() };
        let mut target: Vec<u8> = Vec::new();
        // PE files are always little-endian
        // https://reverseengineering.stackexchange.com/questions/17922/determining-endianness-of-pe-files-windows-on-arm
        target.write_u64::<LittleEndian>(val)?;
        for (idx, elt) in target.iter().enumerate() {
            slc[offset + idx] = *elt;
        }
        Ok(())
    }

    /// Write `val` to `slc` as little-endian at `offset.
    ///
    /// If `Ok` is returned, `self` will have been modified
    /// in-place. Otherwise, no modifications will have been
    /// made.
    pub fn write_u32(&mut self, offset: usize, val: u32) -> Result<()> {
        bounds_check!(offset, self.size);
        let slc = unsafe { self.as_mut_slice() };
        super::write_u32(slc, offset, val)
    }

    /// Read an `i32` from guest memory starting at `offset`
    ///
    /// Return `Ok` with the `i64` value starting at `offset`
    /// if the value between `offset` and `offset + <64 bits>`
    /// was successfully decoded to a little-endian `i64`,
    /// and `Err` otherwise.
    pub fn read_i32(&self, offset: u64) -> Result<i32> {
        bounds_check!(offset, self.size as u64);
        let slc = unsafe { self.as_slice() };
        let mut c = Cursor::new(slc);
        c.set_position(offset);
        c.read_i32::<LittleEndian>().map_err(|e| anyhow!(e))
    }

    /// Write `val` to `slc` as little-endian at `offset.
    ///
    /// If `Ok` is returned, `self` will have been modified
    /// in-place. Otherwise, no modifications will have been
    /// made.
    pub fn write_i32(&mut self, offset: usize, val: i32) -> Result<()> {
        bounds_check!(offset, self.size);
        let slc = unsafe { self.as_mut_slice() };
        let mut target: Vec<u8> = Vec::new();
        target.write_i32::<LittleEndian>(val)?;
        for (idx, elt) in target.iter().enumerate() {
            slc[offset + idx] = *elt;
        }
        Ok(())
    }
}
