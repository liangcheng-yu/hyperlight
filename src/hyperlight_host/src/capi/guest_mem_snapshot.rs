use super::{context::Context, handle::Handle, hdl::Hdl};
use crate::mem::guest_mem_snapshot::GuestMemorySnapshot;
use crate::validate_context;
use anyhow::Result;

mod impls {
    use anyhow::Result;

    use crate::capi::{context::Context, guest_mem::get_guest_memory, handle::Handle, hdl::Hdl};
    use crate::mem::guest_mem_snapshot::GuestMemorySnapshot;

    /// Create a new `GuestMemorySnapshot` from the `GuestMemory` in `ctx`
    /// referenced by `guest_mem_hdl`, then store that new `GuestMemorySnapshot`
    /// in `ctx` and return a new `Handle` referencing it
    pub fn new_snapshot(ctx: &mut Context, guest_mem_hdl: Handle) -> Result<Handle> {
        let gm = get_guest_memory(ctx, guest_mem_hdl)?;
        let snap = GuestMemorySnapshot::new(gm.clone())?;
        Ok(Context::register(
            snap,
            &mut ctx.guest_mem_snapshots,
            Hdl::GuestMemorySnapshot,
        ))
    }

    /// Get the `GuestMemorySnapshot` in `ctx` referenced by
    /// `guest_mem_snapshot_hdl`, then restore the `GuestMemory`
    /// stored therein from the memory snapshot stored therein.
    pub fn restore_from_snapshot(
        ctx: &mut Context,
        guest_mem_snapshot_hdl: Handle,
    ) -> Result<Handle> {
        let snap = super::get_guest_memory_snapshot_mut(ctx, guest_mem_snapshot_hdl)?;
        snap.restore_from_snapshot()?;
        Ok(Handle::new_empty())
    }

    /// Get the `GuestMemorySnapshot` in `ctx` referenced by `guest_mem_snapshot_hdl`,
    /// then call `replace_snapshot()` on it. Return an empty `Handle` on success.
    pub fn replace_snapshot(ctx: &mut Context, guest_mem_snapshot_hdl: Handle) -> Result<Handle> {
        let snap = super::get_guest_memory_snapshot_mut(ctx, guest_mem_snapshot_hdl)?;
        snap.replace_snapshot()?;
        Ok(Handle::new_empty())
    }
}

fn get_guest_memory_snapshot_mut(
    ctx: &mut Context,
    hdl: Handle,
) -> Result<&mut GuestMemorySnapshot> {
    Context::get_mut(hdl, &mut ctx.guest_mem_snapshots, |h| {
        matches!(h, Hdl::GuestMemorySnapshot(_))
    })
}

/// Create a new memory snapshot from the `GuestMemory` in `ctx_ptr`
/// referenced by `guest_mem_hdl`, then store the new snapshot back
/// in `ctx_ptr` and return a `Handle` referencing it.
///
/// If an error occurred, return a `Handle` referencing a new error
/// stored in `ctx_ptr`.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_snapshot_new(
    ctx_ptr: *mut Context,
    guest_mem_hdl: Handle,
) -> Handle {
    validate_context!(ctx_ptr);
    match impls::new_snapshot(&mut *ctx_ptr, guest_mem_hdl) {
        Ok(h) => h,
        Err(e) => (*ctx_ptr).register_err(e),
    }
}

/// Get the `GuestMemorySnapshot` referenced by `guest_mem_snapshot_hdl`
/// from `ctx_ptr`, then restore the `GuestMemory` stored therein from the
/// memory snapshot also stored therein.
///
/// Return an empty `Handle` on success, or a `Handle` referencing a new
/// error stored in `ctx_ptr` on failure.
///
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_snapshot_restore(
    ctx_ptr: *mut Context,
    guest_mem_snapshot_hdl: Handle,
) -> Handle {
    validate_context!(ctx_ptr);
    match impls::restore_from_snapshot(&mut *ctx_ptr, guest_mem_snapshot_hdl) {
        Ok(h) => h,
        Err(e) => (*ctx_ptr).register_err(e),
    }
}

/// /// Get the `GuestMemorySnapshot` referenced by `guest_mem_snapshot_hdl`
/// from `ctx_ptr`, then re-snapshot the `GuestMemory` stored therein and
/// replace the existing snapshot stored therein with the new snapshot.
///
/// Return an empty `Handle` on success, or a `Handle` referencing a new
/// error stored in `ctx_ptr` on failure.
/// # Safety
///
/// You must call this function with a `Context*` that has been:
///
/// - Created with `context_new`
/// - Not yet freed with `context_free`
/// - Not modified, except by calling functions in the Hyperlight C API
#[no_mangle]
pub unsafe extern "C" fn guest_memory_snapshot_replace(
    ctx_ptr: *mut Context,
    guest_mem_snapshot_hdl: Handle,
) -> Handle {
    validate_context!(ctx_ptr);
    match impls::replace_snapshot(&mut *ctx_ptr, guest_mem_snapshot_hdl) {
        Ok(h) => h,
        Err(e) => (*ctx_ptr).register_err(e),
    }
}
