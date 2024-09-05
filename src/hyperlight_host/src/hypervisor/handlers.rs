use std::sync::{Arc, Mutex};

use tracing::{instrument, Span};

use crate::{new_error, Result};

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
pub type OutBHandlerWrapper = Arc<Mutex<dyn OutBHandlerCaller>>;

pub(crate) type OutBHandlerFunction = Box<dyn FnMut(u16, u64) -> Result<()> + Send>;

/// A `OutBHandler` implementation using a `OutBHandlerFunction`
///
/// Note: This handler must live no longer than the `Sandbox` to which it belongs
pub(crate) struct OutBHandler(Arc<Mutex<OutBHandlerFunction>>);

impl From<OutBHandlerFunction> for OutBHandler {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn from(func: OutBHandlerFunction) -> Self {
        Self(Arc::new(Mutex::new(func)))
    }
}

impl OutBHandlerCaller for OutBHandler {
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn call(&mut self, port: u16, payload: u64) -> Result<()> {
        let mut func = self.0.try_lock().map_err(|_| new_error!("Error locking"))?;
        func(port, payload)
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
pub type MemAccessHandlerWrapper = Arc<Mutex<dyn MemAccessHandlerCaller>>;

pub(crate) type MemAccessHandlerFunction = Box<dyn FnMut() -> Result<()> + Send>;

/// A `MemAccessHandler` implementation using `MemAccessHandlerFunction`.
///
/// Note: This handler must live for as long as its Sandbox or for
/// static in the case of its C API usage.
pub(crate) struct MemAccessHandler(Arc<Mutex<MemAccessHandlerFunction>>);

impl From<MemAccessHandlerFunction> for MemAccessHandler {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn from(func: MemAccessHandlerFunction) -> Self {
        Self(Arc::new(Mutex::new(func)))
    }
}

impl MemAccessHandlerCaller for MemAccessHandler {
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn call(&mut self) -> Result<()> {
        let mut func = self.0.try_lock().map_err(|_| new_error!("Error locking"))?;
        func()
    }
}
