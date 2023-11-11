use crate::Result;
use crate::mem::{shared_mem::SharedMemory, layout::SandboxMemoryLayout};

pub(crate) trait WriteFunctionCallToMemory {
    fn write(
        &self,
        function_call_buffer: &[u8],
        guest_mem: &mut SharedMemory,
        layout: &SandboxMemoryLayout,
    ) -> Result<()>;
}

pub(crate) trait ReadFunctionCallFromMemory {
    fn read(&self, guest_mem: &SharedMemory, layout: &SandboxMemoryLayout) -> Result<Vec<u8>>;
}