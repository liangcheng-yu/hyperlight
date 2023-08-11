use anyhow::Result;
use std::sync::{Arc, Mutex};

/// The trait representing custom logic to handle the case when
/// a Hypervisor's virtual CPU (vCPU) informs Hyperlight the guest
/// has initiated an outb operation.
pub trait OutBHandlerCaller {
    fn call(&mut self, port: u16, payload: u64) -> Result<()>;
}

/// A convenient type representing a common way `OutBHandler` implementations
/// are passed as parameters to functions
///
/// Note: This needs to be wrapped in a Mutex to be able to grab a mutable
/// reference to the underlying data (i.e., handle_outb in `Sandbox` takes
/// a &mut self).
pub type OutBHandlerWrapper<'a> = Arc<Mutex<dyn OutBHandlerCaller + 'a>>;

pub(crate) type OutBHandlerFunction<'a> = Box<dyn FnMut(u16, u64) -> Result<()> + 'a>;

/// A `OutBHandler` implementation using a `OutBHandlerFunction`
///
/// Note: This handler must live for as long as the its Sandbox or for
/// static in the case of its C API usage.
pub(crate) struct OutBHandler<'a>(OutBHandlerFunction<'a>);

impl<'a> From<OutBHandlerFunction<'a>> for OutBHandler<'a> {
    fn from(func: OutBHandlerFunction<'a>) -> Self {
        Self(func)
    }
}

impl<'a> OutBHandlerCaller for OutBHandler<'a> {
    fn call(&mut self, port: u16, payload: u64) -> Result<()> {
        (self.0)(port, payload)
    }
}

/// The trait representing custom logic to handle the case when
/// a Hypervisor's virtual CPU (vCPU) informs Hyperlight a memory access
/// outside the designated address space has occured.
pub trait MemAccessHandlerCaller {
    fn call(&mut self) -> Result<()>;
}

/// A convenient type representing a common way `MemAccessHandler` implementations
/// are passed as parameters to functions
///
/// Note: This needs to be wrapped in a Mutex to be able to grab a mutable
/// reference to the underlying data (i.e., handle_mmio_exit in `Sandbox` takes
/// a &mut self).
#[allow(unused)]
pub type MemAccessHandlerWrapper<'a> = Arc<Mutex<dyn MemAccessHandlerCaller + 'a>>;

pub(crate) type MemAccessHandlerFunction<'a> = Box<dyn FnMut() -> Result<()> + 'a>;

/// A `MemAccessHandler` implementation using `MemAccessHandlerFunction`.
///
/// Note: This handler must live for as long as the its Sandbox or for
/// static in the case of its C API usage.
pub(crate) struct MemAccessHandler<'a>(MemAccessHandlerFunction<'a>);

impl<'a> From<MemAccessHandlerFunction<'a>> for MemAccessHandler<'a> {
    fn from(func: MemAccessHandlerFunction<'a>) -> Self {
        Self(func)
    }
}

impl<'a> MemAccessHandlerCaller for MemAccessHandler<'a> {
    fn call(&mut self) -> Result<()> {
        (self.0)()
    }
}
