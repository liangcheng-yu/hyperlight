use std::rc::Rc;

/// The trait representing custom logic to handle the case when
/// a Hypervisor's virtual CPU (vCPU) informs Hyperlight the guest
/// has initiated an outb operation.
pub(crate) trait OutBHandler {
    fn call(&self, port: u16, payload: u64);
}

/// A convenient type representing a common way `OutBHandler` implementations
/// are passed as parameters to functions
///
/// TODO: remove this unused annotation after WHP is rewritten in Rust
#[allow(unused)]
pub(crate) type OutBHandlerRc = Rc<dyn OutBHandler>;

/// A `OutBHandler` implementation using a `Fn`
pub(crate) struct OutBHandlerFn {
    func: Box<dyn Fn(u16, u64)>,
}

impl From<Box<dyn Fn(u16, u64)>> for OutBHandlerFn {
    fn from(func: Box<dyn Fn(u16, u64)>) -> Self {
        Self { func }
    }
}

impl OutBHandler for OutBHandlerFn {
    fn call(&self, port: u16, payload: u64) {
        (self.func)(port, payload)
    }
}

/// The trait representing custom logic to handle the case when
/// a Hypervisor's virtual CPU (vCPU) informs Hyperlight a memory access
/// outside the designated address space has occured.
pub(crate) trait MemAccessHandler {
    fn call(&self);
}

/// A convenient type representing a common way `MemAccessHandler` implementations
/// are passed as parameters to functions
///
/// TODO: remove this unused annotation after WHP is rewritten in Rust
#[allow(unused)]
pub(crate) type MemAccessHandlerRc = Rc<dyn MemAccessHandler>;

/// A `MemAccessHandler` implementation using `Fn`.
pub(crate) struct MemAccessHandlerFn {
    func: Box<dyn Fn()>,
}

impl From<Box<dyn Fn()>> for MemAccessHandlerFn {
    fn from(func: Box<dyn Fn()>) -> Self {
        Self { func }
    }
}

impl MemAccessHandler for MemAccessHandlerFn {
    fn call(&self) {
        (self.func)()
    }
}
