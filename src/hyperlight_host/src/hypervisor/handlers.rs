use crate::{new_error, Result};
use std::sync::{Arc, Mutex};

/// The trait representing custom logic to handle the case when
/// a Hypervisor's virtual CPU (vCPU) informs Hyperlight the guest
/// has initiated an outb operation.
pub trait OutBHandlerCaller: Sync + Send {
    /// Function that gets called when an outb operation has occurred.
    fn call(&mut self, port: u16, payload: u64) -> Result<()>;
}

/// A convenient type representing a common way `OutBHandler` implementations
/// are passed as parameters to functions
///
/// Note: This needs to be wrapped in a Mutex to be able to grab a mutable
/// reference to the underlying data (i.e., handle_outb in `Sandbox` takes
/// a &mut self).
pub type OutBHandlerWrapper<'a> = Arc<Mutex<dyn OutBHandlerCaller + 'a>>;

pub(crate) type OutBHandlerFunction<'a> = Box<dyn FnMut(u16, u64) -> Result<()> + 'a + Send>;

/// A `OutBHandler` implementation using a `OutBHandlerFunction`
///
/// Note: This handler must live no longer than the `Sandbox` to which it belongs
pub(crate) struct OutBHandler<'a>(Arc<Mutex<OutBHandlerFunction<'a>>>);

impl<'a> From<OutBHandlerFunction<'a>> for OutBHandler<'a> {
    fn from(func: OutBHandlerFunction<'a>) -> Self {
        Self(Arc::new(Mutex::new(func)))
    }
}

impl<'a> OutBHandlerCaller for OutBHandler<'a> {
    fn call(&mut self, port: u16, payload: u64) -> Result<()> {
        let mut func = self.0.lock()?;
        (func)(port, payload)
    }
}

/// The trait representing custom logic to handle the case when
/// a Hypervisor's virtual CPU (vCPU) informs Hyperlight a memory access
/// outside the designated address space has occurred.
pub trait MemAccessHandlerCaller: Send {
    /// Function that gets called when unexpected memory access has occurred.
    fn call(&mut self) -> Result<()>;
}

/// A convenient type representing a common way `MemAccessHandler` implementations
/// are passed as parameters to functions
///
/// Note: This needs to be wrapped in a Mutex to be able to grab a mutable
/// reference to the underlying data (i.e., handle_mmio_exit in `Sandbox` takes
/// a &mut self).
pub type MemAccessHandlerWrapper<'a> = Arc<Mutex<dyn MemAccessHandlerCaller + 'a>>;

pub(crate) type MemAccessHandlerFunction<'a> = Box<dyn FnMut() -> Result<()> + 'a + Send>;

/// A `MemAccessHandler` implementation using `MemAccessHandlerFunction`.
///
/// Note: This handler must live for as long as the its Sandbox or for
/// static in the case of its C API usage.
pub(crate) struct MemAccessHandler<'a>(Arc<Mutex<MemAccessHandlerFunction<'a>>>);

impl<'a> From<MemAccessHandlerFunction<'a>> for MemAccessHandler<'a> {
    fn from(func: MemAccessHandlerFunction<'a>) -> Self {
        Self(Arc::new(Mutex::new(func)))
    }
}

impl<'a> MemAccessHandlerCaller for MemAccessHandler<'a> {
    fn call(&mut self) -> Result<()> {
        let mut func = self
            .0
            .lock()
            .map_err(|_| new_error!("could not lock mem access function"))?;
        (func)()
    }
}
