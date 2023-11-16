use super::ptr_offset::Offset;
use super::try_add_ext::UnsafeTryAddExt;
#[cfg(target_os = "linux")]
use crate::error::HyperlightError::MmapFailed;
use crate::error::HyperlightError::{BoundsCheckFailed, IOError, MemoryAllocationFailed};
use crate::log_then_return;
use crate::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[cfg(target_os = "linux")]
use libc::{mmap, munmap};
use std::ffi::c_void;
use std::io::{Cursor, Error};
use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::Arc;
#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_DECOMMIT, PAGE_EXECUTE_READWRITE,
};

// TODO: Check for underflow as well as overflow
macro_rules! bounds_check {
    ($offset:expr, $size:expr) => {
        if $offset > $size {
            log_then_return!(BoundsCheckFailed(u64::from($offset), $size));
        }
    };
}

/// A `std::io::Cursor` for reading byte slices
type ByteSliceCursor<'a> = Cursor<&'a [u8]>;

/// A function that's capable of reading a data of type `T` from a
/// given `Cursor` of bytes.
type Reader<T> = Box<dyn Fn(ByteSliceCursor) -> Result<T>>;
/// A function that's capable of writing a type `T` into
/// a given `Cursor` of bytes.
type Writer<'a, T> = Box<dyn Fn(T) -> Result<Vec<u8>>>;

// SharedMemory needs to send so that it can be used in a Sandbox that can be passed between threads
// *mut c_void is not send, so that means SharedMemory cannot be Send.
// to work around this  we could try and wrap *mut c_void in a Mutex but in order for Mutex to be send
// *mut c_void would need to be sync which it is not.
// Additionally *muc c_void is impl !Send so we cannot unsafe impl Send for *mut c_void
// Therefore we need to wrap *mut c_void in a struct that is impl Send
// We also need to make this type Sync as it is wrapped in an Arc and Arc (just like Mutex) requires Sync in order to impl Send
// Marking this type Sync is safe as it is intended to only ever used from a single thread.

#[derive(Debug)]
/// Pointer to a mutable `c_void` object.
pub struct PtrCVoidMut(*mut c_void);

impl PtrCVoidMut {
    #[cfg(target_os = "windows")]
    pub(crate) fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.0
    }
}

impl From<*mut c_void> for PtrCVoidMut {
    fn from(value: *mut c_void) -> Self {
        Self(value)
    }
}

unsafe impl Send for PtrCVoidMut {}
unsafe impl Sync for PtrCVoidMut {}

#[derive(Debug)]
struct PtrAndSize {
    ptr: PtrCVoidMut,
    size: usize,
}

impl Drop for PtrAndSize {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            unsafe {
                munmap(self.ptr.0, self.size);
            }
        }
        #[cfg(target_os = "windows")]
        {
            unsafe {
                VirtualFree(self.ptr.0, self.size, MEM_DECOMMIT);
            }
        }
    }
}

/// A representation of the guests's physical memory, often referred to as
/// Guest Physical Memory or Guest Physical Addresses (GPA) in Windows
/// Hypervisor Platform.
///
/// `SharedMemory` instances can be cloned inexpensively. Internally,
/// this structure roughly reduces to a reference-counted pointer,
/// so a clone just increases the reference count of the pointer. Beware,
/// however, that only the last clone to be dropped will cause the underlying
/// memory to be freed.
#[derive(Debug)]
pub struct SharedMemory {
    ptr_and_size: Arc<PtrAndSize>,
}

impl Clone for SharedMemory {
    fn clone(&self) -> Self {
        Self {
            ptr_and_size: self.ptr_and_size.clone(),
        }
    }
}

impl SharedMemory {
    /// Create a new region of shared memory with the given minimum
    /// size in bytes.
    ///
    /// Return `Err` if shared memory could not be allocated.
    pub fn new(min_size_bytes: usize) -> Result<Self> {
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
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
                        null_mut(),
                        min_size_bytes as size_t,
                        PROT_READ | PROT_WRITE,
                        MAP_ANONYMOUS | MAP_SHARED | MAP_NORESERVE,
                        -1 as c_int,
                        0 as off_t,
                    );
                    if ptr == MAP_FAILED {
                        log_then_return!(MmapFailed(Error::last_os_error().raw_os_error()));
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
            0 | -1 => {
                log_then_return!(MemoryAllocationFailed(
                    Error::last_os_error().raw_os_error()
                ));
            }
            _ => Ok(Self {
                ptr_and_size: Arc::new(PtrAndSize {
                    ptr: PtrCVoidMut(addr),
                    size: min_size_bytes,
                }),
            }),
        }
    }

    /// Get the base address of shared memory as a `usize`.
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
    pub fn copy_from_slice(&mut self, from_bytes: &[u8], offset: Offset) -> Result<()> {
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
        offset: Offset,
    ) -> Result<()> {
        bounds_check!(offset, self.mem_size());
        let num_bytes = if len > slc.len() { slc.len() } else { len };
        bounds_check!(offset + num_bytes, self.mem_size());
        let dst_ptr = {
            let ptr = self.raw_ptr() as *mut u8;
            ptr.try_add(offset)?
        };

        std::ptr::copy_nonoverlapping(slc.as_ptr(), dst_ptr, num_bytes);

        Ok(())
    }

    /// copy all of `self` in the range `[ offset, offset + slc.len() )`
    /// into `slc` and return `Ok`. If the range is invalid, return `Err`
    ///
    /// # Example usage
    ///
    /// The below will copy 20 bytes from `shared_mem` starting at
    /// the very beginning of the guest memory (offset 0).
    ///
    /// ```rust
    /// # use hyperlight_host::Result;
    /// # use hyperlight_host::mem::shared_mem::SharedMemory;
    /// let mut ret_vec = vec![b'\0'; 20];
    /// let shared_mem = SharedMemory::new(1024).unwrap();
    /// shared_mem.copy_to_slice(ret_vec.as_mut_slice(), 0.into());
    /// ```
    pub fn copy_to_slice(&self, slc: &mut [u8], offset: Offset) -> Result<()> {
        bounds_check!(offset, self.mem_size());
        bounds_check!(offset + slc.len(), self.mem_size());
        let src_ptr = {
            let ptr = self.raw_ptr() as *const u8;
            unsafe {
                // safety: we know ptr+offset is owned by `ptr`
                ptr.try_add(offset)?
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
    pub(crate) fn copy_all_to_vec(&self) -> Result<Vec<u8>> {
        // ensure this vec has _length_ (not just capacity) of self.mem_size()
        // so that the copy_to_slice reads the slice length properly.
        let mut ret_vec = vec![b'\0'; self.mem_size()];
        self.copy_to_slice(ret_vec.as_mut_slice(), Offset::zero())?;
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
    pub(crate) fn raw_ptr(&self) -> *mut c_void {
        self.ptr_and_size.ptr.0
    }

    /// Return the length of the memory contained in `self`.
    ///
    /// The return value is guaranteed to be the size of memory
    /// of which `self.raw_ptr()` points to the beginning.
    pub fn mem_size(&self) -> usize {
        self.ptr_and_size.size
    }

    /// Return the address of memory at an offset to this `SharedMemory` checking
    /// that the memory is within the bounds of the `SharedMemory`.
    pub(crate) fn calculate_address(&self, offset: Offset) -> Result<usize> {
        bounds_check!(offset, self.mem_size());
        usize::try_from(self.base_addr() + offset)
    }

    /// Read an `u64` from shared memory starting at `offset`
    ///
    /// Return `Ok` with the `u64` value starting at `offset`
    /// if the value between `offset` and `offset + <64 bits>`
    /// was successfully decoded to a little-endian `u64`,
    /// and `Err` otherwise.
    pub(crate) fn read_u64(&self, offset: Offset) -> Result<u64> {
        self.read(
            offset,
            Box::new(|mut c| c.read_u64::<LittleEndian>().map_err(IOError)),
        )
    }

    /// Read an `i32` from shared memory starting at `offset`
    ///
    /// Return `Ok` with the `i64` value starting at `offset`
    /// if the value between `offset` and `offset + <64 bits>`
    /// was successfully decoded to a little-endian `i64`,
    /// and `Err` otherwise.
    pub fn read_i32(&self, offset: Offset) -> Result<i32> {
        self.read(
            offset,
            Box::new(|mut c| c.read_i32::<LittleEndian>().map_err(IOError)),
        )
    }

    /// Read a `u32` from shared memory starting at `offset`
    ///
    /// Return `Ok` with the `u32` value starting at `offset`
    /// if the value in the bit range `[offset, <offset + 32 bits>)`
    /// was successfully decoded to a `u8`, and `Err` otherwise.
    pub(crate) fn read_u32(&self, offset: Offset) -> Result<u32> {
        self.read(
            offset,
            Box::new(|mut c| c.read_u32::<LittleEndian>().map_err(IOError)),
        )
    }

    /// Read a value of type T from the memory in `self`, using `reader`
    /// to do the conversion from bytes to the actual type
    fn read<T>(&self, offset: Offset, reader: Reader<T>) -> Result<T> {
        bounds_check!(offset, self.mem_size());
        bounds_check!(offset + size_of::<T>() as u64, self.mem_size());
        let slc = unsafe { self.as_slice() };
        let mut c = Cursor::new(slc);
        c.set_position(offset.into());
        reader(c)
    }

    /// Write `val` into shared memory at the given offset
    /// from the start of shared memory
    pub fn write_u64(&mut self, offset: Offset, val: u64) -> Result<()> {
        self.write(
            offset,
            val,
            Box::new(|val| {
                let mut v = Vec::new();
                v.write_u64::<LittleEndian>(val)?;
                Ok(v)
            }),
        )
    }

    /// Write `val` to shared memory as little-endian at `offset`.
    ///
    /// If `Ok` is returned, `self` will have been modified
    /// in-place. Otherwise, no modifications will have been
    /// made.
    pub fn write_i32(&mut self, offset: Offset, val: i32) -> Result<()> {
        self.write(
            offset,
            val,
            Box::new(|val| {
                let mut v = Vec::new();
                v.write_i32::<LittleEndian>(val)?;
                Ok(v)
            }),
        )
    }

    /// Write `val` to `offset` in `self` by doing the following:
    ///
    /// 1. call `writer(val)` to get a `Vec<u8>` representation of `val`
    /// 2. Write the resulting `Vec<u8>` into `self`, starting at `offset`,
    /// in order
    ///
    /// Returns `Ok` iff the call in (1) succeeded, and all the write operations
    /// in (2) succeeded. Otherwise, returns `Err`. If `Ok` is returned,
    /// some but not all writes may have been done.
    fn write<T>(&mut self, offset: Offset, val: T, writer: Writer<T>) -> Result<()> {
        bounds_check!(offset, self.mem_size());
        bounds_check!(offset + size_of::<T>(), self.mem_size());
        let slc = unsafe { self.as_mut_slice() };
        let target = writer(val)?;
        for (idx, elt) in target.iter().enumerate() {
            let slc_idx: usize = (offset + idx).try_into()?;
            slc[slc_idx] = *elt;
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
}

#[cfg(test)]
mod tests {
    use super::SharedMemory;
    use crate::error::HyperlightError::IOError;
    use crate::mem::ptr_offset::Offset;
    use crate::mem::shared_mem_tests::read_write_test_suite;
    use crate::Result;
    use byteorder::ReadBytesExt;
    use proptest::prelude::*;

    /// Read a `u8` (i.e. a byte) from shared memory starting at `offset`
    ///
    /// Return `Ok` with the `u8` value starting at `offset`
    /// if the value in the range `[offset, offset + 8)`
    /// was successfully decoded to a `u8`, and `Err` otherwise.
    fn read_u8(shared_mem: &SharedMemory, offset: Offset) -> Result<u8> {
        shared_mem.read(offset, Box::new(|mut c| c.read_u8().map_err(IOError)))
    }

    #[test]
    fn copy_to_from_slice() {
        let mem_size: usize = 4096;
        let vec_len = 10;
        let mut gm = SharedMemory::new(mem_size).unwrap();
        let vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        // write the value to the memory at the beginning.
        gm.copy_from_slice(&vec, Offset::zero()).unwrap();

        let mut vec2 = vec![0; vec_len];
        // read the value back from the memory at the beginning.
        gm.copy_to_slice(&mut vec2, Offset::zero()).unwrap();
        assert_eq!(vec, vec2);

        let offset = Offset::try_from(mem_size - vec.len()).unwrap();
        // write the value to the memory at the end.
        unsafe { gm.copy_from_slice_subset(&vec, vec.len(), offset) }.unwrap();

        let mut vec3 = vec![0; vec_len];
        // read the value back from the memory at the end.
        gm.copy_to_slice(&mut vec3, offset).unwrap();
        assert_eq!(vec, vec3);

        let offset = Offset::try_from(mem_size / 2).unwrap();
        // write the value to the memory at the middle.
        unsafe { gm.copy_from_slice_subset(&vec, vec.len(), offset) }.unwrap();

        let mut vec4 = vec![0; vec_len];
        // read the value back from the memory at the middle.
        gm.copy_to_slice(&mut vec4, offset).unwrap();
        assert_eq!(vec, vec4);

        // try and read a value from an offset that is beyond the end of the memory.
        let mut vec5 = vec![0; vec_len];
        assert!(gm
            .copy_to_slice(&mut vec5, Offset::try_from(mem_size).unwrap())
            .is_err());

        // try and write a value to an offset that is beyond the end of the memory.
        assert!(unsafe {
            gm.copy_from_slice_subset(&vec, vec.len(), Offset::try_from(mem_size).unwrap())
        }
        .is_err());

        // try and read a value from an offset that is too large.
        let mut vec6 = vec![0; vec_len];
        assert!(gm
            .copy_to_slice(&mut vec6, Offset::try_from(mem_size * 2).unwrap())
            .is_err());

        // try and write a value to an offset that is too large.
        assert!(unsafe {
            gm.copy_from_slice_subset(&vec, vec.len(), Offset::try_from(mem_size * 2).unwrap())
        }
        .is_err());

        // try and read a value that is too large.
        let mut vec7 = vec![0; mem_size * 2];
        let len = vec7.len();
        assert!(gm
            .copy_to_slice(&mut vec7, Offset::try_from(mem_size * 2).unwrap())
            .is_err());

        // try and write a value that is too large.
        assert!(unsafe {
            gm.copy_from_slice_subset(&vec7, len, Offset::try_from(mem_size * 2).unwrap())
        }
        .is_err());
    }

    #[test]
    fn copy_into_from() -> Result<()> {
        let mem_size: usize = 4096;
        let vec_len = 10;
        let mut gm = SharedMemory::new(mem_size)?;
        let vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        // write the value to the memory at the beginning.
        gm.copy_from_slice(&vec, Offset::zero())?;

        let mut vec2 = vec![0; vec_len];
        // read the value back from the memory at the beginning.
        gm.copy_to_slice(vec2.as_mut_slice(), Offset::zero())?;
        assert_eq!(vec, vec2);

        let offset = Offset::try_from(mem_size - vec.len()).unwrap();
        // write the value to the memory at the end.
        gm.copy_from_slice(&vec, offset)?;

        let mut vec3 = vec![0; vec_len];
        // read the value back from the memory at the end.
        gm.copy_to_slice(&mut vec3, offset)?;
        assert_eq!(vec, vec3);

        let offset = Offset::try_from(mem_size / 2).unwrap();
        // write the value to the memory at the middle.
        gm.copy_from_slice(&vec, offset)?;

        let mut vec4 = vec![0; vec_len];
        // read the value back from the memory at the middle.
        gm.copy_to_slice(&mut vec4, offset)?;
        assert_eq!(vec, vec4);

        // try and read a value from an offset that is beyond the end of the memory.
        let mut vec5 = vec![0; vec_len];
        assert!(gm
            .copy_to_slice(&mut vec5, Offset::try_from(mem_size).unwrap())
            .is_err());

        // try and write a value to an offset that is beyond the end of the memory.
        assert!(gm
            .copy_from_slice(&vec5, Offset::try_from(mem_size).unwrap())
            .is_err());

        // try and read a value from an offset that is too large.
        let mut vec6 = vec![0; vec_len];
        assert!(gm
            .copy_to_slice(&mut vec6, Offset::try_from(mem_size * 2).unwrap())
            .is_err());

        // try and write a value to an offset that is too large.
        assert!(gm
            .copy_from_slice(&vec6, Offset::try_from(mem_size * 2).unwrap())
            .is_err());

        // try and read a value that is too large.
        let mut vec7 = vec![0; mem_size * 2];
        assert!(gm.copy_to_slice(&mut vec7, Offset::zero()).is_err());

        // try and write a value that is too large.
        assert!(gm.copy_from_slice(&vec7, Offset::zero()).is_err());

        Ok(())
    }

    proptest! {
        #[test]
        fn read_write_i32(mem_size in 256_usize..4096_usize, val in -0x1000_i32..0x1000_i32) {
            read_write_test_suite(
                mem_size,
                val,
                Box::new(SharedMemory::read_i32),
                Box::new(SharedMemory::write_i32),
            )
            .unwrap();
        }
    }

    #[test]
    fn alloc_fail() {
        let gm = SharedMemory::new(0);
        assert!(gm.is_err());
        let gm = SharedMemory::new(usize::MAX);
        assert!(gm.is_err());
    }

    const MIN_SIZE: usize = 123;
    #[test]
    fn clone() {
        let mut gm1 = SharedMemory::new(MIN_SIZE).unwrap();
        let mut gm2 = gm1.clone();

        // after gm1 is cloned, gm1 and gm2 should have identical
        // memory sizes and pointers.
        assert_eq!(gm1.mem_size(), gm2.mem_size());
        assert_eq!(gm1.raw_ptr(), gm2.raw_ptr());

        // we should be able to copy a byte array into both gm1 and gm2,
        // and have both changes be reflected in all clones
        gm1.copy_from_slice(&[b'a'], Offset::zero()).unwrap();
        gm2.copy_from_slice(&[b'b'], Offset::from(1_u64)).unwrap();

        // at this point, both gm1 and gm2 should have
        // offset 0 = 'a', offset 1 = 'b'
        for (raw_offset, expected) in &[(0_u64, b'a'), (1_u64, b'b')] {
            let offset = Offset::from(*raw_offset);
            assert_eq!(read_u8(&gm1, offset).unwrap(), *expected);
            assert_eq!(read_u8(&gm2, offset).unwrap(), *expected);
        }

        // after we drop gm1, gm2 should still exist, be valid,
        // and have all contents from before gm1 was dropped
        drop(gm1);

        // at this point, gm2 should still have offset 0 = 'a', offset 1 = 'b'
        for (raw_offset, expected) in &[(0_u64, b'a'), (1_u64, b'b')] {
            let offset = Offset::from(*raw_offset);
            assert_eq!(read_u8(&gm2, offset).unwrap(), *expected);
        }
        gm2.copy_from_slice(&[b'c'], Offset::from(2_u64)).unwrap();
        assert_eq!(read_u8(&gm2, Offset::from(2_u64)).unwrap(), b'c');
        drop(gm2);
    }

    #[test]
    fn copy_all_to_vec() {
        let data = vec![b'a', b'b', b'c'];
        let mut gm = SharedMemory::new(data.len()).unwrap();
        gm.copy_from_slice(data.as_slice(), Offset::from(0_u64))
            .unwrap();
        let ret_vec = gm.copy_all_to_vec().unwrap();
        assert_eq!(data, ret_vec);
    }

    /// A test to ensure that, if a `SharedMem` instance is cloned
    /// and _all_ clones are dropped, the memory region will no longer
    /// be valid.
    ///
    /// This test is ignored because it is incompatible with other tests as
    /// they may be allocating memory at the same time.
    ///
    /// Marking this test as ignored means that running `cargo test` will not
    /// run it. This feature will allow a developer who runs that command
    /// from their workstation to be successful without needing to know about
    /// test interdependencies. This test will, however, be run explcitly as a
    /// part of the CI pipeline.
    #[test]
    #[ignore]
    #[cfg(target_os = "linux")]
    fn test_drop() {
        let pid = std::process::id();

        let address_in_ranges = |addr: usize, maps: Vec<proc_maps::MapRange>| {
            maps.iter().any(|map| {
                let start = map.start();
                let end = start + map.size();
                start <= addr && addr <= end
            })
        };

        let sm1 = SharedMemory::new(MIN_SIZE).unwrap();
        let sm2 = sm1.clone();
        let addr = sm1.raw_ptr() as usize;
        // ensure the address is in the process's virtual memory
        let maps = proc_maps::get_process_maps(pid.try_into().unwrap()).unwrap();
        assert!(
            address_in_ranges(addr, maps),
            "shared memory address {:#x} was not found in process map, but should be",
            addr,
        );
        // drop both shared memory instances, which should result
        // in freeing the memory region
        drop(sm1);
        drop(sm2);

        let maps = proc_maps::get_process_maps(pid.try_into().unwrap()).unwrap();
        // now, ensure the address is not in the process's virtual memory
        assert!(
            address_in_ranges(addr, maps),
            "shared memory address {:#x} was found in the process map, but shouldn't be",
            addr
        );
    }
}

#[cfg(test)]
mod prop_tests {
    use crate::mem::ptr_offset::Offset;

    use super::SharedMemory;
    use proptest::prelude::*;

    proptest! {
        /// Test the `copy_from_slice`, `copy_to_slice`, `copy_all_to_slice`,
        /// and `as_slice` methods on shared memory
        ///
        /// Note about parameters: if you modify any of the below parameters,
        /// ensure the following holds for all combinations:
        ///
        /// (offset + slice_size) < (slice_size * size_multiplier)
        #[test]
        fn slice_ops(
            slice_size in 1_usize..500_usize,
            slice_val in b'\0'..b'z',
        ) {
            let offset = Offset::try_from(slice_size / 2).unwrap();
            let mut shared_mem = {
                let mem_size = usize::try_from(slice_size + offset).unwrap();
                SharedMemory::new(mem_size).unwrap()
            };

            let orig_vec = {
                let the_vec = vec![slice_val; slice_size];
                shared_mem.copy_from_slice(the_vec.as_slice(), offset).unwrap();
                the_vec
            };

            {
                // test copy_to_slice
                let mut ret_vec = vec![slice_val + 1; slice_size];
                shared_mem.copy_to_slice(ret_vec.as_mut_slice(), offset).unwrap();
                assert_eq!(orig_vec, ret_vec);
            }

            {
                // test copy_all_to_vec
                let ret_vec = shared_mem.copy_all_to_vec().unwrap();
                assert_eq!(orig_vec, ret_vec[usize::try_from(offset).unwrap()..usize::try_from(offset+slice_size).unwrap()]);
            }

            {
                // test as_slice
                let total_slice = unsafe { shared_mem.as_slice()};
                let range_start: usize = offset.try_into().unwrap();
                let range_end: usize = (offset + slice_size).try_into().unwrap();
                assert_eq!(orig_vec.as_slice(), &total_slice[range_start..range_end])
            }

            {
                // test as_mut_slice
                let total_slice = unsafe { shared_mem.as_mut_slice() };
                let range_start: usize = offset.try_into().unwrap();
                let range_end: usize = (offset + slice_size).try_into().unwrap();
                assert_eq!(orig_vec.as_slice(), &total_slice[range_start..range_end]);
            }
        }
    }
}
