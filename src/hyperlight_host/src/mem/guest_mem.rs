use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[cfg(target_os = "linux")]
use libc::{mmap, munmap};
use std::ffi::c_void;
use std::io::{Cursor, Error};
use std::mem::size_of;
use std::ptr::null_mut;
use std::rc::Rc;
#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_DECOMMIT, PAGE_EXECUTE_READWRITE,
};

// TODO: Check for underflow as well as overflow
macro_rules! bounds_check {
    ($offset:expr, $size:expr) => {
        if $offset > $size {
            anyhow::bail!(
                "offset {} out of bounds (max size: size {})",
                $offset,
                $size,
            );
        }
    };
}

/// A representation of the guests's physical memory, often referred to as
/// Guest Physical Memory or Guest Physical Addresses (GPA) in Windows
/// Hypervisor Platform.
///
/// `GuestMemory` instances can be cloned inexpensively. Internally,
/// this structure roughly reduces to a reference-counted pointer,
/// so a clone just increases the reference count of the pointer. Beware,
/// however, that only the last clone to be dropped will cause the underlying
/// memory to be freed.
#[derive(Debug)]
pub struct GuestMemory {
    ptr_and_size: Rc<(*mut c_void, usize)>,
}

impl Clone for GuestMemory {
    fn clone(&self) -> Self {
        Self {
            ptr_and_size: self.ptr_and_size.clone(),
        }
    }
}

impl Drop for GuestMemory {
    fn drop(&mut self) {
        // if this GuestMemory has been cloned, or is a clone
        // of some other GuestMemory, don't actually free
        // the underlying memory map
        //
        // Note: regardless which case we're in, the return value
        // of strong_count() will equal $TOTAL_NUM_CLONES + 1
        if Rc::strong_count(&self.ptr_and_size) > 1 {
            return;
        }
        let (ptr, size) = *self.ptr_and_size;
        #[cfg(target_os = "linux")]
        {
            unsafe {
                munmap(ptr, size);
            }
        }
        #[cfg(target_os = "windows")]
        {
            unsafe {
                VirtualFree(ptr, size, MEM_DECOMMIT);
            }
        }
    }
}

impl GuestMemory {
    /// Create a new region of guest memory with the given minimum
    /// size in bytes.
    ///
    /// Return `Err` if guest memory could not be allocated.
    pub fn new(min_size_bytes: usize) -> Result<Self> {
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                use anyhow::bail;
                use libc::{
                    size_t,
                    c_int,
                    off_t,
                    PROT_READ,
                    PROT_WRITE,
                    MAP_ANONYMOUS,
                    MAP_SHARED,
                    MAP_NORESERVE,
                    MAP_FAILED,
                };
                // https://docs.rs/libc/latest/libc/fn.mmap.html
                let addr = unsafe {
                    let ptr = mmap(
                        null_mut() as *mut c_void,
                        min_size_bytes as size_t,
                        PROT_READ | PROT_WRITE,
                        MAP_ANONYMOUS | MAP_SHARED | MAP_NORESERVE,
                        -1 as c_int,
                        0 as off_t,
                    );
                    if ptr == MAP_FAILED {
                        bail!(std::io::Error::last_os_error())
                    }
                    ptr
                };
            } else {
                let addr = unsafe {
                    // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Memory/fn.VirtualAlloc.html
                    VirtualAlloc(
                        Some(null_mut() as *mut c_void),
                        min_size_bytes,
                        MEM_COMMIT,
                        PAGE_EXECUTE_READWRITE,
                    )
                };
            }
        }
        match addr as i64 {
            0 | -1 => anyhow::bail!("Memory Allocation Failed Error {}", Error::last_os_error()),
            _ => Ok(Self {
                ptr_and_size: Rc::new((addr, min_size_bytes)),
            }),
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
        self.raw_ptr() as usize
    }

    /// If all memory locations within the range
    /// `[offset, offset + from_bytes.len()]` are valid, copy all
    /// bytes from `from_bytes` in order to `self` and return `Ok`.
    /// Otherwise, return `Err`.
    pub fn copy_from_slice(&mut self, from_bytes: &[u8], offset: usize) -> Result<()> {
        bounds_check!(offset, self.mem_size());
        bounds_check!(offset + from_bytes.len(), self.mem_size());
        unsafe { self.copy_from_slice_subset(from_bytes, from_bytes.len(), offset) }
    }

    /// Copies bytes from `slc[0]` to `slc[len]` into the memory to
    /// which `self` points
    unsafe fn copy_from_slice_subset(
        &mut self,
        slc: &[u8],
        len: usize,
        offset: usize,
    ) -> Result<()> {
        bounds_check!(offset, self.mem_size());
        let num_bytes = if len > slc.len() { slc.len() } else { len };
        bounds_check!(offset + num_bytes, self.mem_size());
        let dst_ptr = {
            let ptr = self.raw_ptr() as *mut u8;
            ptr.add(offset)
        };

        std::ptr::copy_nonoverlapping(slc.as_ptr(), dst_ptr, num_bytes);

        Ok(())
    }

    /// copy all of `self` in the range `[ offset, offset + slc.len() )`
    /// into `slc` and return `Ok`. If the range is invalid, return `Err`
    ///
    /// # Example usage
    ///
    /// The below will copy 20 bytes from `guest_mem` starting at
    /// the very beginning of the guest memory (offset 0).
    ///
    /// ```rust
    /// let mut ret_vec = vec![b'\0'; 20];
    /// guest_mem.copy_to_slice(ret_vec.as_mut_slice(), 0)?
    /// ```
    pub fn copy_to_slice(&self, slc: &mut [u8], offset: usize) -> Result<()> {
        bounds_check!(offset, self.mem_size());
        bounds_check!(offset + slc.len(), self.mem_size());
        let src_ptr = {
            let ptr = self.raw_ptr() as *const u8;
            unsafe {
                // safety: we know offset is owned by `ptr`
                ptr.add(offset)
            }
        };

        unsafe {
            // safety: we've checked bounds and produced `src_ptr`
            // ourselves
            std::ptr::copy(src_ptr, slc.as_mut_ptr(), slc.len())
        };

        Ok(())
    }

    /// Copy the entire contents of `self` into a `Vec<u8>`, then return it
    pub fn copy_all_to_vec(&self) -> Result<Vec<u8>> {
        // ensure this vec has _length_ (not just capacity) of self.mem_size()
        // so that the copy_to_slice reads the slice length properly.
        let mut ret_vec = vec![b'\0'; self.mem_size()];
        self.copy_to_slice(ret_vec.as_mut_slice(), 0)?;
        Ok(ret_vec)
    }

    /// Get the raw pointer to the memory region.
    ///
    /// # Safety
    ///
    /// The pointer will point to a shared memory region
    /// of size `self.size`. The caller must ensure that any
    /// writes they do directly to this pointer fall completely
    /// within this region, and that they do not attempt to
    /// free any of this memory, since it is owned and will
    /// be cleaned up by `self`.
    pub fn raw_ptr(&self) -> *mut c_void {
        let (ptr, _) = *self.ptr_and_size;
        ptr
    }

    /// Return the length of the memory contained in `self`.
    ///
    /// The return value is guaranteed to be the size of memory
    /// of which `self.raw_ptr()` points to the beginning.
    pub fn mem_size(&self) -> usize {
        let (_, size) = *self.ptr_and_size;
        size
    }

    /// Return the address of memory at an offset to this GuestMemory checking
    /// that the memory is within the bounds of the GuestMemory.
    pub fn calculate_address(&self, offset: usize) -> Result<usize> {
        bounds_check!(offset, self.mem_size());
        Ok(self.base_addr() + offset)
    }

    /// Read an `i64` from guest memory starting at `offset`
    ///
    /// Return `Ok` with the `i64` value starting at `offset`
    /// if the value between `offset` and `offset + <64 bits>`
    /// was successfully decoded to a little-endian `i64`,
    /// and `Err` otherwise.
    pub fn read_i64(&self, offset: u64) -> Result<i64> {
        bounds_check!(offset, self.mem_size() as u64);
        bounds_check!(offset + size_of::<i64>() as u64, self.mem_size() as u64);
        let slc = unsafe { self.as_slice() };
        let mut c = Cursor::new(slc);
        c.set_position(offset);
        c.read_i64::<LittleEndian>().map_err(|e| anyhow!(e))
    }

    /// Write val into guest memory at the given offset
    /// from the start of guest memory
    pub fn write_u64(&mut self, offset: usize, val: u64) -> Result<()> {
        bounds_check!(offset, self.mem_size());
        bounds_check!(offset + size_of::<u64>(), self.mem_size());
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
        std::slice::from_raw_parts_mut(self.raw_ptr() as *mut u8, self.mem_size())
    }

    unsafe fn as_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.raw_ptr() as *const u8, self.mem_size())
    }

    /// Read an `i32` from guest memory starting at `offset`
    ///
    /// Return `Ok` with the `i64` value starting at `offset`
    /// if the value between `offset` and `offset + <64 bits>`
    /// was successfully decoded to a little-endian `i64`,
    /// and `Err` otherwise.
    pub fn read_i32(&self, offset: u64) -> Result<i32> {
        bounds_check!(offset, self.mem_size() as u64);
        bounds_check!(offset + size_of::<i32>() as u64, self.mem_size() as u64);
        let slc = unsafe { self.as_slice() };
        let mut c = Cursor::new(slc);
        c.set_position(offset);
        c.read_i32::<LittleEndian>().map_err(|e| anyhow!(e))
    }

    /// Read a `u8` (i.e. a byte) from guest memory starting at `offset`
    ///
    /// Return `Ok` with the `u8` value starting at `offset`
    /// if the value in the range `[offset, offset + 8)`
    /// was successfully decoded to a `u8`, and `Err` otherwise.
    pub fn read_u8(&self, offset: u64) -> Result<u8> {
        bounds_check!(offset, self.mem_size() as u64);
        let slc = unsafe { self.as_slice() };
        let mut c = Cursor::new(slc);
        c.set_position(offset);
        c.read_u8().map_err(|e| anyhow!(e))
    }

    /// Write `val` to `slc` as little-endian at `offset.
    ///
    /// If `Ok` is returned, `self` will have been modified
    /// in-place. Otherwise, no modifications will have been
    /// made.
    pub fn write_i32(&mut self, offset: usize, val: i32) -> Result<()> {
        bounds_check!(offset, self.mem_size());
        bounds_check!(offset + size_of::<i32>(), self.mem_size());
        let slc = unsafe { self.as_mut_slice() };
        let mut target: Vec<u8> = Vec::new();
        target.write_i32::<LittleEndian>(val)?;
        for (idx, elt) in target.iter().enumerate() {
            slc[offset + idx] = *elt;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::GuestMemory;
    use anyhow::Result;
    #[cfg(target_os = "linux")]
    use libc::{mmap, munmap};
    use std::ffi::c_void;
    use std::mem::size_of;
    #[cfg(target_os = "windows")]
    use windows::Win32::System::Memory::{
        VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_DECOMMIT, PAGE_EXECUTE_READWRITE,
    };

    #[test]
    pub fn copy_to_from_slice() -> Result<()> {
        let mem_size: usize = 4096;
        let vec_len = 10;
        let mut gm = GuestMemory::new(mem_size)?;
        let vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        // write the value to the memory at the beginning.
        gm.copy_from_slice(&vec, 0)?;

        let mut vec2 = vec![0; vec_len];
        // read the value back from the memory at the beginning.
        gm.copy_to_slice(&mut vec2, 0)?;
        assert_eq!(vec, vec2);

        let offset = mem_size - vec.len();
        // write the value to the memory at the end.
        unsafe { gm.copy_from_slice_subset(&vec, vec.len(), offset)? };

        let mut vec3 = vec![0; vec_len];
        // read the value back from the memory at the end.
        gm.copy_to_slice(&mut vec3, offset)?;
        assert_eq!(vec, vec3);

        let offset = mem_size / 2;
        // write the value to the memory at the middle.
        unsafe { gm.copy_from_slice_subset(&vec, vec.len(), offset)? };

        let mut vec4 = vec![0; vec_len];
        // read the value back from the memory at the middle.
        gm.copy_to_slice(&mut vec4, offset)?;
        assert_eq!(vec, vec4);

        // try and read a value from an offset that is beyond the end of the memory.
        let mut vec5 = vec![0; vec_len];
        assert!(gm.copy_to_slice(&mut vec5, mem_size).is_err());

        // try and write a value to an offset that is beyond the end of the memory.
        assert!(unsafe { gm.copy_from_slice_subset(&vec, vec.len(), mem_size) }.is_err());

        // try and read a value from an offset that is too large.
        let mut vec6 = vec![0; vec_len];
        assert!(gm.copy_to_slice(&mut vec6, mem_size * 2).is_err());

        // try and write a value to an offset that is too large.
        assert!(unsafe { gm.copy_from_slice_subset(&vec, vec.len(), mem_size * 2) }.is_err());

        // try and read a value that is too large.
        let mut vec7 = vec![0; mem_size * 2];
        let len = vec7.len();
        assert!(gm.copy_to_slice(&mut vec7, mem_size * 2).is_err());

        // try and write a value that is too large.
        assert!(unsafe { gm.copy_from_slice_subset(&vec7, len, mem_size * 2) }.is_err());

        Ok(())
    }

    #[test]
    pub fn copy_into_from() -> Result<()> {
        let mem_size: usize = 4096;
        let vec_len = 10;
        let mut gm = GuestMemory::new(mem_size)?;
        let vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        // write the value to the memory at the beginning.
        gm.copy_from_slice(&vec, 0)?;

        let mut vec2 = vec![0; vec_len];
        // read the value back from the memory at the beginning.
        gm.copy_to_slice(vec2.as_mut_slice(), 0)?;
        assert_eq!(vec, vec2);

        let offset = mem_size - vec.len();
        // write the value to the memory at the end.
        gm.copy_from_slice(&vec, offset)?;

        let mut vec3 = vec![0; vec_len];
        // read the value back from the memory at the end.
        gm.copy_to_slice(&mut vec3, offset)?;
        assert_eq!(vec, vec3);

        let offset = mem_size / 2;
        // write the value to the memory at the middle.
        gm.copy_from_slice(&vec, offset)?;

        let mut vec4 = vec![0; vec_len];
        // read the value back from the memory at the middle.
        gm.copy_to_slice(&mut vec4, offset)?;
        assert_eq!(vec, vec4);

        // try and read a value from an offset that is beyond the end of the memory.
        let mut vec5 = vec![0; vec_len];
        assert!(gm.copy_to_slice(&mut vec5, mem_size).is_err());

        // try and write a value to an offset that is beyond the end of the memory.
        assert!(gm.copy_from_slice(&vec5, mem_size).is_err());

        // try and read a value from an offset that is too large.
        let mut vec6 = vec![0; vec_len];
        assert!(gm.copy_to_slice(&mut vec6, mem_size * 2).is_err());

        // try and write a value to an offset that is too large.
        assert!(gm.copy_from_slice(&vec6, mem_size * 2).is_err());

        // try and read a value that is too large.
        let mut vec7 = vec![0; mem_size * 2];
        assert!(gm.copy_to_slice(&mut vec7, 0).is_err());

        // try and write a value that is too large.
        assert!(gm.copy_from_slice(&vec7, 0).is_err());

        Ok(())
    }

    #[test]
    pub fn read_write_i32() -> Result<()> {
        let mem_size: usize = 4096;
        let mut gm = GuestMemory::new(mem_size)?;
        let val: i32 = 42;
        // write the value to the memory at the beginning.
        gm.write_i32(0, val)?;
        // read the value back from the memory at the beginning.
        let read_val = gm.read_i32(0)?;
        assert_eq!(val, read_val);

        let offset = mem_size - size_of::<i32>();
        // write the value to the memory at the end.
        gm.write_i32(offset, val)?;
        // read the value back from the memory at the end.
        let read_val = gm.read_i32(offset as u64)?;
        assert_eq!(val, read_val);

        let offset = mem_size / 2;
        // write the value to the memory at the middle.
        gm.write_i32(offset, val)?;
        // read the value back from the memory at the middle.
        let read_val = gm.read_i32(offset as u64)?;
        assert_eq!(val, read_val);

        // try and read a value from the memory at an invalid offset.
        let result = gm.read_i32((mem_size * 2) as u64);
        assert!(result.is_err());

        // try and write the value to the memory at an invalid offset.
        let result = gm.write_i32(mem_size * 2, val);
        assert!(result.is_err());

        // try and read a value from the memory beyond the end of the memory.
        let result = gm.read_i32(mem_size as u64);
        assert!(result.is_err());

        // try and write the value to the memory beyond the end of the memory.
        let result = gm.write_i32(mem_size, val);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    pub fn read_i64_write_u64() -> Result<()> {
        let mem_size: usize = 4096;
        let mut gm = GuestMemory::new(mem_size)?;
        let val: i64 = 42;
        // write the value to the memory at the beginning.
        gm.write_u64(0, val.try_into().unwrap())?;
        // read the value back from the memory at the beginning.
        let read_val = gm.read_i64(0)?;
        assert_eq!(val, read_val);

        let offset = mem_size - size_of::<i64>();
        // write the value to the memory at the end.
        gm.write_u64(offset, val.try_into().unwrap())?;
        // read the value back from the memory at the end.
        let read_val = gm.read_i64(offset as u64)?;
        assert_eq!(val, read_val);

        let offset = mem_size / 2;
        // write the value to the memory at the middle.
        gm.write_u64(offset, val.try_into().unwrap())?;
        // read the value back from the memory at the middle.
        let read_val = gm.read_i64(offset as u64)?;
        assert_eq!(val, read_val);

        // try and read a value from the memory at an invalid offset.
        let result = gm.read_i64((mem_size * 2) as u64);
        assert!(result.is_err());

        // try and write the value to the memory at an invalid offset.
        let result = gm.write_u64(mem_size * 2, val.try_into().unwrap());
        assert!(result.is_err());

        // try and read a value from the memory beyond the end of the memory.
        let result = gm.read_i64(mem_size as u64);
        assert!(result.is_err());

        // try and write the value to the memory beyond the end of the memory.
        let result = gm.write_u64(mem_size, val.try_into().unwrap());
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    pub fn alloc_fail() -> Result<()> {
        let gm = GuestMemory::new(0);
        assert!(gm.is_err());
        let gm = GuestMemory::new(usize::MAX);
        assert!(gm.is_err());
        Ok(())
    }

    const MIN_SIZE: usize = 123;
    #[test]
    pub fn clone() {
        let mut gm1 = GuestMemory::new(MIN_SIZE).unwrap();
        let mut gm2 = gm1.clone();

        // after gm1 is cloned, gm1 and gm2 should have identical
        // memory sizes and pointers.
        assert_eq!(gm1.mem_size(), gm2.mem_size());
        assert_eq!(gm1.raw_ptr(), gm2.raw_ptr());

        // we should be able to copy a byte array into both gm1 and gm2,
        // and have both changes be reflected in all clones
        gm1.copy_from_slice(&[b'a'], 0).unwrap();
        gm2.copy_from_slice(&[b'b'], 1).unwrap();

        // at this point, both gm1 and gm2 should have
        // offset 0 = 'a', offset 1 = 'b'
        for (offset, expected) in &[(0, b'a'), (1, b'b')] {
            assert_eq!(gm1.read_u8(*offset).unwrap(), *expected);
            assert_eq!(gm2.read_u8(*offset).unwrap(), *expected);
        }

        // after we drop gm1, gm2 should still exist, be valid,
        // and have all contents from before gm1 was dropped
        drop(gm1);

        // at this point, gm2 should still have offset 0 = 'a', offset 1 = 'b'
        for (offset, expected) in &[(0, b'a'), (1, b'b')] {
            assert_eq!(gm2.read_u8(*offset).unwrap(), *expected);
        }
        gm2.copy_from_slice(&[b'c'], 2).unwrap();
        assert_eq!(gm2.read_u8(2).unwrap(), b'c');
        drop(gm2);
    }

    #[test]
    pub fn copy_all_to_vec() {
        let data = vec![b'a', b'b', b'c'];
        let mut gm = GuestMemory::new(data.len()).unwrap();
        gm.copy_from_slice(data.as_slice(), 0).unwrap();
        let ret_vec = gm.copy_all_to_vec().unwrap();
        assert_eq!(data, ret_vec);
    }

    #[test]
    pub fn test_drop() -> Result<()> {
        let addr: *mut c_void;
        let size: usize;
        {
            let gm = GuestMemory::new(MIN_SIZE)?;
            addr = gm.raw_ptr();
            size = gm.mem_size();
        };

        // guest memory should be dropped at this point,
        // another attempt to memory-map the same address
        // at which it was allocated should succeed, because
        // that address should have previously been freed.
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                // on Linux, mmap only takes the address (first param)
                // as a hint, but only guarantees that it'll not
                // return NULL if the call succeeded.
                let mmap_addr = unsafe {mmap(
                    addr,
                    size,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
                    -1,
                    0,
                )};
                assert_ne!(std::ptr::null_mut(), mmap_addr);
                assert_eq!(0, unsafe{munmap(addr, size)});
            } else if #[cfg(windows)] {
                let valloc_addr = unsafe {
                    VirtualAlloc(
                        Some(addr),
                        size,
                        MEM_COMMIT,
                        PAGE_EXECUTE_READWRITE,
                    )
                };
                assert_ne!(std::ptr::null_mut(), valloc_addr);
                assert_eq!(true, unsafe{
                    VirtualFree(addr, 0, MEM_DECOMMIT)
                });
            }
        };
        Ok(())
    }
}
