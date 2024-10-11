use std::fs::File;
use std::io::Read;
use std::vec::Vec;

use super::pe::headers::PEHeaders;
use super::pe::pe_info::PEInfo;
use super::ptr_offset::Offset;
use crate::Result;

pub enum ExeInfo {
    PE(PEInfo),
}

impl ExeInfo {
    pub fn from_file(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        Self::from_buf(&contents)
    }
    pub fn from_buf(buf: &[u8]) -> Result<Self> {
        PEInfo::new(buf).map(ExeInfo::PE)
    }
    pub fn stack_reserve(&self) -> u64 {
        match self {
            ExeInfo::PE(pe) => pe.stack_reserve(),
        }
    }
    pub fn heap_reserve(&self) -> u64 {
        match self {
            ExeInfo::PE(pe) => pe.heap_reserve(),
        }
    }
    pub fn entrypoint(&self) -> Offset {
        match self {
            ExeInfo::PE(pe) => Offset::from(PEHeaders::from(pe).entrypoint_offset),
        }
    }
    pub fn loaded_size(&self) -> usize {
        match self {
            ExeInfo::PE(pe) => pe.payload.len(),
        }
    }
    // todo: this doesn't morally need to be &mut self, since we're
    // copying into target, but the PE loader chooses to apply
    // relocations in its owned representation of the PE contents,
    // which requires it to be &mut.
    pub fn load(&mut self, load_addr: usize, target: &mut [u8]) -> Result<()> {
        match self {
            ExeInfo::PE(pe) => {
                let patches = pe.get_exe_relocation_patches(load_addr)?;
                pe.apply_relocation_patches(patches)?;
                target[0..pe.payload.len()].copy_from_slice(&pe.payload);
            }
        }
        Ok(())
    }
}
