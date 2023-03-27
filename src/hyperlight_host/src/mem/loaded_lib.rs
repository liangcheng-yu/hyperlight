use std::ffi::CString;

use super::ptr::RawPtr;
use anyhow::Result;
use windows::core::PCSTR;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::{FreeLibrary, LoadLibraryA};

#[cfg(target_os = "windows")]
pub struct LoadedLib {
    h_instance: HINSTANCE,
    file_name_c_str: *mut u8,
}

impl LoadedLib {
    pub(crate) fn base_addr(&self) -> Result<RawPtr> {
        let h_inst_u64: u64 = self.h_instance.0.try_into()?;
        Ok(RawPtr::from(h_inst_u64))
    }
}

impl Drop for LoadedLib {
    fn drop(&mut self) {
        // the library, referenced by self.h_instance, owns
        // self.file_name. make sure they're freed in reverse order
        unsafe {
            FreeLibrary(self.h_instance);
            drop(CString::from_raw(self.file_name_c_str as *mut i8));
        }
    }
}

impl TryFrom<&str> for LoadedLib {
    type Error = anyhow::Error;
    fn try_from(file_name: &str) -> Result<Self> {
        let cstr = CString::new(file_name)?.into_raw() as *mut u8;
        let file_name_pc_str = PCSTR::from_raw(cstr as *const u8);
        let h_instance = unsafe { LoadLibraryA(file_name_pc_str) }?;
        Ok(Self {
            h_instance,
            file_name_c_str: cstr,
        })
    }
}
