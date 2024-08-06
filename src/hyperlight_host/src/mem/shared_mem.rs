use std::any::type_name;
use std::ffi::c_void;
use std::io::Error;
use std::ptr::null_mut;
use std::sync::Arc;

use hyperlight_common::mem::PAGE_SIZE_USIZE;
use tracing::{instrument, Span};
#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT, PAGE_EXECUTE_READWRITE};

use crate::{log_then_return, new_error, Result};

/// Makes sure that the given `offset` and `size` are within the bounds of the memory with size `mem_size`.
macro_rules! bounds_check {
    ($offset:expr, $size:expr, $mem_size:expr) => {
        if $offset + $size > $mem_size {
            return Err(new_error!(
                "Cannot read value from offset {} with size {} in memory of size {}",
                $offset,
                $size,
                $mem_size
            ));
        }
    };
}

/// generates a reader function for the given type
macro_rules! generate_reader {
    ($fname:ident, $ty:ty) => {
        /// Read a value of type `$ty` from the memory at the given offset.
        #[allow(dead_code)]
        #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
        pub(crate) fn $fname(&self, offset: usize) -> Result<$ty> {
            let data = self.as_mut_slice();
            bounds_check!(offset, std::mem::size_of::<$ty>(), data.len());
            Ok(<$ty>::from_le_bytes(
                data[offset..offset + std::mem::size_of::<$ty>()].try_into()?,
            ))
        }
    };
}

/// generates a writer function for the given type
macro_rules! generate_writer {
    ($fname:ident, $ty:ty) => {
        /// Write a value of type `$ty` to the memory at the given offset.
        #[allow(dead_code)]
        #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
        pub(crate) fn $fname(&mut self, offset: usize, value: $ty) -> Result<()> {
            let data = self.as_mut_slice();
            bounds_check!(offset, std::mem::size_of::<$ty>(), data.len());
            data[offset..offset + std::mem::size_of::<$ty>()].copy_from_slice(&value.to_le_bytes());
            Ok(())
        }
    };
}

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
    #[cfg(target_os = "linux")]
    fn drop(&mut self) {
        use libc::munmap;

        unsafe {
            munmap(self.ptr.0, self.size);
        }
    }
    #[cfg(target_os = "windows")]
    fn drop(&mut self) {
        use windows::Win32::System::Memory::{VirtualFree, MEM_DECOMMIT};

        unsafe {
            VirtualFree(self.ptr.0, self.size, MEM_DECOMMIT);
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

impl<'a> SharedMemory {
    /// Create a new region of shared memory with the given minimum
    /// size in bytes. The region will be surrounded by guard pages.
    ///
    /// Return `Err` if shared memory could not be allocated.
    #[cfg(target_os = "linux")]
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn new(min_size_bytes: usize) -> Result<Self> {
        use libc::{
            c_int, mmap, mprotect, off_t, size_t, MAP_ANONYMOUS, MAP_FAILED, MAP_NORESERVE,
            MAP_SHARED, PROT_NONE, PROT_READ, PROT_WRITE,
        };

        use crate::error::HyperlightError::{MmapFailed, MprotectFailed};

        if min_size_bytes == 0 {
            return Err(new_error!("Cannot create shared memory with size 0"));
        }

        let total_size = min_size_bytes
            .checked_add(2 * PAGE_SIZE_USIZE) // guard page around the memory
            .ok_or_else(|| new_error!("Memory required for sandbox exceeded usize::MAX"))?;

        assert!(
            total_size % PAGE_SIZE_USIZE == 0,
            "shared memory must be a multiple of 4096"
        );

        // allocate the memory
        let addr = unsafe {
            let ptr = mmap(
                null_mut(),
                total_size as size_t,
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

        // protect the guard pages
        unsafe {
            let res = mprotect(addr, PAGE_SIZE_USIZE, PROT_NONE);
            if res != 0 {
                return Err(MprotectFailed(Error::last_os_error().raw_os_error()));
            }
            let res = mprotect(
                (addr as *const u8).add(total_size - PAGE_SIZE_USIZE) as *mut c_void,
                PAGE_SIZE_USIZE,
                PROT_NONE,
            );
            if res != 0 {
                return Err(MprotectFailed(Error::last_os_error().raw_os_error()));
            }
        }

        Ok(Self {
            ptr_and_size: Arc::new(PtrAndSize {
                ptr: PtrCVoidMut(addr),
                size: total_size,
            }),
        })
    }

    /// Create a new region of shared memory with the given minimum
    /// size in bytes. The region will be surrounded by guard pages.
    ///
    /// Return `Err` if shared memory could not be allocated.
    #[cfg(target_os = "windows")]
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn new(min_size_bytes: usize) -> Result<Self> {
        use windows::Win32::System::Memory::PAGE_READWRITE;

        use crate::HyperlightError::MemoryAllocationFailed;

        if min_size_bytes == 0 {
            return Err(new_error!("Cannot create shared memory with size 0"));
        }

        let total_size = min_size_bytes
            .checked_add(2 * PAGE_SIZE_USIZE)
            .ok_or_else(|| new_error!("Memory required for sandbox exceeded {}", usize::MAX))?;

        if total_size % PAGE_SIZE_USIZE != 0 {
            return Err(new_error!(
                "shared memory must be a multiple of {}",
                PAGE_SIZE_USIZE
            ));
        }

        // allocate the memory
        let addr = unsafe {
            let ptr = VirtualAlloc(Some(null_mut()), total_size, MEM_COMMIT, PAGE_READWRITE);
            if ptr.is_null() {
                log_then_return!(MemoryAllocationFailed(
                    Error::last_os_error().raw_os_error()
                ));
            }
            ptr
        };

        // TODO protect the guard pages

        Ok(Self {
            ptr_and_size: Arc::new(PtrAndSize {
                ptr: PtrCVoidMut(addr),
                size: total_size,
            }),
        })
    }

    pub(super) fn make_memory_executable(&self) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS};

            let mut _old_flags = PAGE_PROTECTION_FLAGS::default();
            let res = unsafe {
                VirtualProtect(
                    self.ptr_and_size.ptr.0,
                    self.ptr_and_size.size,
                    PAGE_EXECUTE_READWRITE,
                    &mut _old_flags as *mut PAGE_PROTECTION_FLAGS,
                )
            };

            if !res.as_bool() {
                return Err(new_error!(
                    "Failed to make memory executable: {:#?}",
                    Error::last_os_error().raw_os_error()
                ));
            }
        }

        // TODO: Add support for Linux if in process execution is re-enabled on Linux in future

        Ok(())
    }

    /// Internal helper method to get the backing memory as a mutable slice.
    ///
    /// # Safety
    /// Uses unsafe code, but the function signature is not unsafe because
    /// self.base_addr() is guaranteed to be a valid pointer, and
    /// bytes [ self.base_addr() .. self.base_addr() + self.mem_size() )
    /// are guaranteed to be valid for the duration of the lifetime of self.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn as_mut_slice(&self) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.base_addr() as *mut u8, self.mem_size()) }
    }

    /// Internal helper method to get the backing memory as a slice.
    ///
    /// # Safety
    /// Uses unsafe code, but the function signature is not unsafe because
    /// self.base_addr() is guaranteed to be a valid pointer, and
    /// bytes [ self.base_addr() .. self.base_addr() + self.mem_size() )
    /// are guaranteed to be valid for the duration of the lifetime of self.
    #[cfg(test)]
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn as_slice(&self) -> &'a [u8] {
        unsafe { std::slice::from_raw_parts(self.base_addr() as *const u8, self.mem_size()) }
    }

    /// Get the base address of shared memory as a `usize`,
    /// which is the first usable byte of the shared memory.
    ///
    /// # Safety
    ///
    /// This function should not be used to do pointer artithmetic.
    /// Only use it to get the base address of the memory map so you
    /// can do things like calculate offsets, etc...
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn base_addr(&self) -> usize {
        self.raw_ptr() as usize + PAGE_SIZE_USIZE
    }

    /// Return the length of usable memory contained in `self`.
    /// The returned size does not include the size of the surrounding
    /// guard pages.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn mem_size(&self) -> usize {
        self.ptr_and_size.size - 2 * PAGE_SIZE_USIZE
    }

    /// Get the raw pointer to the memory region. This pointer will point
    /// into the first guard page of the shared memory region.
    ///
    /// # Safety
    ///
    /// The pointer will point to a shared memory region
    /// of size `self.size`. The caller must ensure that any
    /// writes they do directly to this pointer fall completely
    /// within this region, and that they do not attempt to
    /// free any of this memory, since it is owned and will
    /// be cleaned up by `self`.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn raw_ptr(&self) -> *mut c_void {
        self.ptr_and_size.ptr.0
    }

    /// Returns the total raw size of the memory region, including the guard pages.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn raw_mem_size(&self) -> usize {
        self.ptr_and_size.size
    }

    /// Copies all bytes from `src` to `self` starting at offset
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn copy_from_slice(&mut self, src: &[u8], offset: usize) -> Result<()> {
        let data = self.as_mut_slice();
        bounds_check!(offset, src.len(), data.len());
        data[offset..offset + src.len()].copy_from_slice(src);
        Ok(())
    }

    /// Fill the memory in the range `[offset, offset + len)` with `value`
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn fill(&mut self, value: u8, offset: usize, len: usize) -> Result<()> {
        let data = self.as_mut_slice();
        bounds_check!(offset, len, data.len());
        data[offset..offset + len].fill(value);
        Ok(())
    }

    /// copy all of `self` in the range `[offset, offset + slc.len())`
    /// into `dst`
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn copy_to_slice(&self, dst: &mut [u8], offset: usize) -> Result<()> {
        let data = self.as_mut_slice();
        bounds_check!(offset, dst.len(), data.len());
        dst.copy_from_slice(&data[offset..offset + dst.len()]);
        Ok(())
    }

    /// Copies the first `len` bytes of `src` into `self`` starting at `offset`
    #[cfg(test)]
    fn copy_from_slice_subset(&mut self, src: &[u8], len: usize, offset: usize) -> Result<()> {
        let data = self.as_mut_slice();
        bounds_check!(offset, len, data.len());
        data[offset..offset + len].copy_from_slice(&src[..len]);
        Ok(())
    }

    /// Copy the entire contents of `self` into a `Vec<u8>`, then return it
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn copy_all_to_vec(&self) -> Result<Vec<u8>> {
        let data = self.as_mut_slice();
        Ok(data.to_vec())
    }

    /// Return the address of memory at an offset to this `SharedMemory` checking
    /// that the memory is within the bounds of the `SharedMemory`.
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn calculate_address(&self, offset: usize) -> Result<usize> {
        bounds_check!(offset, 0, self.mem_size());
        Ok(self.base_addr() + offset)
    }

    generate_reader!(read_u8, u8);
    generate_reader!(read_i8, i8);
    generate_reader!(read_u16, u16);
    generate_reader!(read_i16, i16);
    generate_reader!(read_u32, u32);
    generate_reader!(read_i32, i32);
    generate_reader!(read_u64, u64);
    generate_reader!(read_i64, i64);
    generate_reader!(read_usize, usize);
    generate_reader!(read_isize, isize);

    generate_writer!(write_u8, u8);
    generate_writer!(write_i8, i8);
    generate_writer!(write_u16, u16);
    generate_writer!(write_i16, i16);
    generate_writer!(write_u32, u32);
    generate_writer!(write_i32, i32);
    generate_writer!(write_u64, u64);
    generate_writer!(write_i64, i64);
    generate_writer!(write_usize, usize);
    generate_writer!(write_isize, isize);

    /// Pushes the given data onto shared memory to the buffer at the given offset.
    /// NOTE! buffer_start_offset must point to the beginning of the buffer
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn push_buffer(
        &mut self,
        buffer_start_offset: usize,
        buffer_size: usize,
        data: &[u8],
    ) -> Result<()> {
        let stack_pointer_rel = self.read_u64(buffer_start_offset).unwrap() as usize;
        let buffer_size_u64: u64 = buffer_size.try_into()?;

        if stack_pointer_rel > buffer_size || stack_pointer_rel < 8 {
            return Err(new_error!(
                "Unable to push data to buffer: Stack pointer is out of bounds. Stack pointer: {}, Buffer size: {}",
                stack_pointer_rel,
                buffer_size_u64
            ));
        }

        let size_required = data.len() + 8;
        let size_available = buffer_size - stack_pointer_rel;

        if size_required > size_available {
            return Err(new_error!(
                "Not enough space in buffer to push data. Required: {}, Available: {}",
                size_required,
                size_available
            ));
        }

        // get absolute
        let stack_pointer_abs = stack_pointer_rel + buffer_start_offset;

        // write the actual data to the top of stack
        self.copy_from_slice(data, stack_pointer_abs)?;

        // write the offset to the newly written data, to the top of stack.
        // this is used when popping the stack, to know how far back to jump
        self.write_u64(stack_pointer_abs + data.len(), stack_pointer_rel as u64)?;

        // update stack pointer to point to the next free address
        self.write_u64(
            buffer_start_offset,
            (stack_pointer_rel + data.len() + 8) as u64,
        )
    }

    /// Pops the given given buffer into a `T` and returns it.
    /// NOTE! the data must be a size-prefixed flatbuffer, and
    /// buffer_start_offset must point to the beginning of the buffer
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub fn try_pop_buffer_into<T>(
        &mut self,
        buffer_start_offset: usize,
        buffer_size: usize,
    ) -> Result<T>
    where
        T: for<'b> TryFrom<&'b [u8]>,
    {
        // get the stackpointer
        let stack_pointer_rel = self.read_u64(buffer_start_offset)? as usize;

        if stack_pointer_rel > buffer_size || stack_pointer_rel < 16 {
            return Err(new_error!(
                "Unable to pop data from buffer: Stack pointer is out of bounds. Stack pointer: {}, Buffer size: {}",
                stack_pointer_rel,
                buffer_size
            ));
        }

        // make it absolute
        let last_element_offset_abs = stack_pointer_rel + buffer_start_offset;

        // go back 8 bytes to get offset to element on top of stack
        let last_element_offset_rel: usize =
            self.read_u64(last_element_offset_abs - 8).unwrap() as usize;

        // make it absolute
        let last_element_offset_abs = last_element_offset_rel + buffer_start_offset;

        // Get the size of the flatbuffer buffer from memory
        let fb_buffer_size = {
            let size_i32 = self.read_u32(last_element_offset_abs)? + 4;
            // ^^^ flatbuffer byte arrays are prefixed by 4 bytes
            // indicating its size, so, to get the actual size, we need
            // to add 4.
            usize::try_from(size_i32)
        }?;

        let mut result_buffer = vec![0; fb_buffer_size];

        self.copy_to_slice(&mut result_buffer, last_element_offset_abs)?;
        let to_return = T::try_from(result_buffer.as_slice()).map_err(|_e| {
            new_error!(
                "pop_buffer_into: failed to convert buffer to {}",
                type_name::<T>()
            )
        })?;

        // update the stack pointer to point to the element we just popped off since that is now free
        self.write_u64(buffer_start_offset, last_element_offset_rel as u64)?;

        // zero out the memory we just popped off
        let num_bytes_to_zero = stack_pointer_rel - last_element_offset_rel;
        self.fill(0, last_element_offset_abs, num_bytes_to_zero)?;

        Ok(to_return)
    }
}

#[cfg(test)]
mod tests {
    use hyperlight_common::mem::PAGE_SIZE_USIZE;
    use proptest::prelude::*;

    use super::SharedMemory;
    use crate::mem::shared_mem_tests::read_write_test_suite;
    use crate::Result;

    #[test]
    fn fill() {
        let mem_size: usize = 4096;
        let mut gm = SharedMemory::new(mem_size).unwrap();
        gm.fill(1, 0, 1024).unwrap();
        gm.fill(2, 1024, 1024).unwrap();
        gm.fill(3, 2048, 1024).unwrap();
        gm.fill(4, 3072, 1024).unwrap();

        let vec = gm.copy_all_to_vec().unwrap();

        assert!(vec[0..1024].iter().all(|&x| x == 1));
        assert!(vec[1024..2048].iter().all(|&x| x == 2));
        assert!(vec[2048..3072].iter().all(|&x| x == 3));
        assert!(vec[3072..4096].iter().all(|&x| x == 4));

        gm.fill(5, 0, 4096).unwrap();

        let vec2 = gm.copy_all_to_vec().unwrap();
        assert!(vec2.iter().all(|&x| x == 5));

        assert!(gm.fill(0, 0, mem_size + 1).is_err());
        assert!(gm.fill(0, mem_size, 1).is_err());
    }

    #[test]
    fn copy_to_from_slice() {
        let mem_size: usize = 4096;
        let vec_len = 10;
        let mut gm = SharedMemory::new(mem_size).unwrap();
        let vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        // write the value to the memory at the beginning.
        gm.copy_from_slice(&vec, 0).unwrap();

        let mut vec2 = vec![0; vec_len];
        // read the value back from the memory at the beginning.
        gm.copy_to_slice(&mut vec2, 0).unwrap();
        assert_eq!(vec, vec2);

        let offset = mem_size - vec.len();
        // write the value to the memory at the end.
        gm.copy_from_slice_subset(&vec, vec.len(), offset).unwrap();

        let mut vec3 = vec![0; vec_len];
        // read the value back from the memory at the end.
        gm.copy_to_slice(&mut vec3, offset).unwrap();
        assert_eq!(vec, vec3);

        let offset = mem_size / 2;
        // write the value to the memory at the middle.
        gm.copy_from_slice_subset(&vec, vec.len(), offset).unwrap();

        let mut vec4 = vec![0; vec_len];
        // read the value back from the memory at the middle.
        gm.copy_to_slice(&mut vec4, offset).unwrap();
        assert_eq!(vec, vec4);

        // try and read a value from an offset that is beyond the end of the memory.
        let mut vec5 = vec![0; vec_len];
        assert!(gm.copy_to_slice(&mut vec5, mem_size).is_err());

        // try and write a value to an offset that is beyond the end of the memory.
        assert!(gm
            .copy_from_slice_subset(&vec, vec.len(), mem_size)
            .is_err());

        // try and read a value from an offset that is too large.
        let mut vec6 = vec![0; vec_len];
        assert!(gm.copy_to_slice(&mut vec6, mem_size * 2).is_err());

        // try and write a value to an offset that is too large.
        assert!(gm
            .copy_from_slice_subset(&vec, vec.len(), mem_size * 2)
            .is_err());

        // try and read a value that is too large.
        let mut vec7 = vec![0; mem_size * 2];
        let len = vec7.len();
        assert!(gm.copy_to_slice(&mut vec7, mem_size * 2).is_err());

        // try and write a value that is too large.
        assert!(gm.copy_from_slice_subset(&vec7, len, mem_size * 2).is_err());
    }

    #[test]
    fn copy_into_from() -> Result<()> {
        let mem_size: usize = 4096;
        let vec_len = 10;
        let mut gm = SharedMemory::new(mem_size)?;
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

    proptest! {
        #[test]
        fn read_write_i32(val in -0x1000_i32..0x1000_i32) {
            read_write_test_suite(
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

    #[test]
    fn clone() {
        let mut gm1 = SharedMemory::new(PAGE_SIZE_USIZE).unwrap();
        let mut gm2 = gm1.clone();

        // after gm1 is cloned, gm1 and gm2 should have identical
        // memory sizes and pointers.
        assert_eq!(gm1.mem_size(), gm2.mem_size());
        assert_eq!(gm1.base_addr(), gm2.base_addr());

        // we should be able to copy a byte array into both gm1 and gm2,
        // and have both changes be reflected in all clones
        gm1.copy_from_slice(&[b'a'], 0).unwrap();
        gm2.copy_from_slice(&[b'b'], 1).unwrap();

        // at this point, both gm1 and gm2 should have
        // offset 0 = 'a', offset 1 = 'b'
        for (raw_offset, expected) in &[(0, b'a'), (1, b'b')] {
            assert_eq!(gm1.read_u8(*raw_offset).unwrap(), *expected);
            assert_eq!(gm2.read_u8(*raw_offset).unwrap(), *expected);
        }

        // after we drop gm1, gm2 should still exist, be valid,
        // and have all contents from before gm1 was dropped
        drop(gm1);

        // at this point, gm2 should still have offset 0 = 'a', offset 1 = 'b'
        for (raw_offset, expected) in &[(0, b'a'), (1, b'b')] {
            assert_eq!(gm2.read_u8(*raw_offset).unwrap(), *expected);
        }
        gm2.copy_from_slice(&[b'c'], 2).unwrap();
        assert_eq!(gm2.read_u8(2).unwrap(), b'c');
        drop(gm2);
    }

    #[test]
    fn copy_all_to_vec() {
        let mut data = vec![b'a', b'b', b'c'];
        data.resize(4096, 0);
        let mut gm = SharedMemory::new(data.len()).unwrap();
        gm.copy_from_slice(data.as_slice(), 0).unwrap();
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
        use proc_maps::maps_contain_addr;

        let pid = std::process::id();

        let sm1 = SharedMemory::new(PAGE_SIZE_USIZE).unwrap();
        let sm2 = sm1.clone();
        let addr = sm1.raw_ptr() as usize;

        // ensure the address is in the process's virtual memory
        let maps_before_drop = proc_maps::get_process_maps(pid.try_into().unwrap()).unwrap();
        assert!(
            maps_contain_addr(addr, &maps_before_drop),
            "shared memory address {:#x} was not found in process map, but should be",
            addr,
        );
        // drop both shared memory instances, which should result
        // in freeing the memory region
        drop(sm1);
        drop(sm2);

        let maps_after_drop = proc_maps::get_process_maps(pid.try_into().unwrap()).unwrap();
        // now, ensure the address is not in the process's virtual memory
        assert!(
            !maps_contain_addr(addr, &maps_after_drop),
            "shared memory address {:#x} was found in the process map, but shouldn't be",
            addr
        );
    }

    #[cfg(target_os = "linux")]
    mod guard_page_crash_test {
        use crate::mem::shared_mem::SharedMemory;

        const TEST_EXIT_CODE: u8 = 211; // an uncommon exit code, used for testing purposes

        /// hook sigsegv to exit with status code, to make it testable, rather than have it exit from a signal
        /// NOTE: We CANNOT panic!() in the handler, and make the tests #[should_panic], because
        ///     the test harness process will crash anyway after the test passes
        fn setup_signal_handler() {
            unsafe {
                signal_hook_registry::register_signal_unchecked(libc::SIGSEGV, || {
                    std::process::exit(TEST_EXIT_CODE.into());
                })
                .unwrap();
            }
        }

        #[test]
        #[ignore] // this test is ignored because it will crash the running process
        fn read() {
            setup_signal_handler();

            let mem = SharedMemory::new(4096).unwrap();
            let guard_page_ptr = mem.raw_ptr();
            unsafe { std::ptr::read_volatile(guard_page_ptr) };
        }

        #[test]
        #[ignore] // this test is ignored because it will crash the running process
        fn write() {
            setup_signal_handler();

            let mem = SharedMemory::new(4096).unwrap();
            let guard_page_ptr = mem.raw_ptr();
            unsafe { std::ptr::write_volatile(guard_page_ptr as *mut u8, 0u8) };
        }

        #[test]
        #[ignore] // this test is ignored because it will crash the running process
        fn exec() {
            setup_signal_handler();

            let mem = SharedMemory::new(4096).unwrap();
            let guard_page_ptr = mem.raw_ptr();
            let func: fn() = unsafe { std::mem::transmute(guard_page_ptr) };
            func();
        }

        // provides a way for running the above tests in a separate process since they expect to crash
        #[test]
        fn guard_page_testing_shim() {
            let tests = vec!["read", "write", "exec"];

            for test in tests {
                let status = std::process::Command::new("cargo")
                    .args(["test", "-p", "hyperlight_host", "--", "--ignored", test])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .expect("Unable to launch tests");
                assert_eq!(
                    status.code(),
                    Some(TEST_EXIT_CODE.into()),
                    "Guard Page test failed: {}",
                    test
                );
            }
        }
    }
}

#[cfg(test)]
mod prop_tests {
    use hyperlight_common::mem::PAGE_SIZE_USIZE;
    use proptest::prelude::*;

    use super::SharedMemory;

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
            let offset = slice_size / 2;
            let mut shared_mem = SharedMemory::new(PAGE_SIZE_USIZE).unwrap();

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
                assert_eq!(orig_vec, ret_vec[offset..offset+slice_size]);
            }

            {
                // test as_slice
                let total_slice =  shared_mem.as_slice();
                assert_eq!(orig_vec.as_slice(), &total_slice[offset..offset + slice_size])
            }

            {
                // test as_mut_slice
                let total_slice =  shared_mem.as_mut_slice() ;
                assert_eq!(orig_vec.as_slice(), &total_slice[offset..offset + slice_size]);
            }
        }
    }
}
