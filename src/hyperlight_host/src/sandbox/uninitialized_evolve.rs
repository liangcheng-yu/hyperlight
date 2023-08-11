use super::mem_mgr::MemMgr;
#[cfg(target_os = "windows")]
use crate::func::exports::get_os_page_size;
#[cfg(target_os = "windows")]
use crate::mem::ptr::RawPtr;
use crate::{
    hypervisor::handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper},
    Sandbox, UninitializedSandbox,
};
#[cfg(target_os = "linux")]
use anyhow::bail;
use anyhow::Result;
use tracing::instrument;

#[allow(unused)]
#[instrument(err(Debug), skip_all)]
pub(super) fn evolve_impl<'a>(
    mut u_sbox: UninitializedSandbox<'a>,
    outb_hdl: OutBHandlerWrapper,
    mem_access_hdl: MemAccessHandlerWrapper,
) -> Result<Sandbox<'a>> {
    let run_from_proc_mem = u_sbox.run_from_process_memory;
    if run_from_proc_mem {
        evolve_in_proc(u_sbox, outb_hdl)
    } else {
        let mem_mgr = {
            // we are gonna borrow u_sbox mutably below in our
            // get_hypervisor_mut call, so we need to borrow it
            // immutably here, and disallow that borrow to escape this scope
            // so we can do the mutable borrow later.
            //
            // luckily, cloning SandboxMemoryManagers is cheap, so we can do
            // that and avoid the borrow going out of this scope by moving the
            // clone
            let mgr = u_sbox.get_mem_mgr();
            mgr.clone()
        };

        u_sbox
            .hv
            .initialise(&mem_mgr, outb_hdl.clone(), mem_access_hdl.clone())
            .unwrap();
        Ok(Sandbox::from(u_sbox))
    }
}

fn evolve_in_proc(
    mut u_sbox: UninitializedSandbox<'_>,
    outb_hdl: OutBHandlerWrapper,
) -> Result<Sandbox<'_>> {
    #[cfg(target_os = "linux")]
    {
        // Note from old C# implementation of this function:
        //
        // This code is unstable, it causes segmentation faults so for now we
        // are throwing an exception if we try to run in process in Linux.
        // I think this is due to the fact that the guest binary is built for
        // windows x64 compilation for windows uses fastcall which is different
        // on windows and linux dotnet will default to the calling convention
        // for the platform that the code is running on.
        // so we need to set the calling convention to the one for which the
        // guest binary is build (windows x64 calling convention docs:
        // https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention?view=msvc-170
        // ).
        // on linux however, this isn't possible (see this document for more
        // details: https://docs.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.callingconvention?view=net-6.0
        //.
        //
        // Alternatives:
        //
        // 1. build the binary for windows and linux and then run the correct
        // version for the platform on which we're running.
        //
        // 2. alter the calling convention of the guest binary and then tell
        // dotnet to use that calling convention. the only option for this
        // seems to be vectorcall
        // (https://docs.microsoft.com/en-us/cpp/cpp/vectorcall?view=msvc-170).
        // cdecl and stdcall are not possible using CL on x64 platform.
        // vectorcall is not supported by dotnet
        // (see https://github.com/dotnet/runtime/issues/8300)
        //
        // 3. write our own code to correct the calling convention
        //
        // 4. write epilog/prolog code in the guest binary.
        //
        // also see https://www.agner.org/optimize/calling_conventions.pdf
        // and https://eli.thegreenplace.net/2011/09/06/stack-frame-layout-on-x86-64/

        // the following lines are here to ensure clippy/rustc doesn't
        // complain about the following parameters:
        //
        // - u_sbox being marked mut and unused
        // - outb_hdl being unused
        let _ = u_sbox.get_mem_mgr_mut();
        let _ = outb_hdl;
        bail!("in-process execution is not supported on linux");
    }
    #[cfg(target_os = "windows")]
    {
        /// Get a C-compatible function pointer for the given outb_hdl.
        /// This function is purposely declared within this compile-time
        /// flag because it should only be used herein
        /// (really, it should never be used, but we have to do so here).
        ///
        /// Generally speaking, `outb_hdl_as_fn_ptr` does some pointer
        /// tricks to get a pointer to something C-compatible that looks
        /// like a `void (u16, u64)` to C, so we can then set that address
        /// as the outb handler function and call the entry point.
        ///
        /// See https://stackoverflow.com/a/38997480 for the detail that
        /// inspired this method, and
        /// https://stackoverflow.com/questions/32270030/how-do-i-convert-a-rust-closure-to-a-c-style-callback
        /// for some additional information.
        ///
        /// Additionally, there are explanatory comments inside the function's
        /// implementation.
        fn outb_hdl_as_fn_ptr(outb_hdl: OutBHandlerWrapper) -> *const (u16, u64) {
            use std::os::raw::c_void;

            // first, we need to define a closure that calls outb_hdl.call.
            // this is actually a necessary step because Rust distinguishes,
            // in the type system, between a closure and a Fn/FnOnce/FnMut.
            //
            // The former is a closure, while the latter is a trait object.
            // This is an important distinction because the latter has a
            // "fat" pointer that contains a reference to both the executable
            // code and the context over which the original closure closes.
            let closure = |port: u16, payload: u64| outb_hdl.call(port, payload);
            // Now we're coercing to a trait object, which means the compiler
            // guarantees we have a "fat" pointer that contains both a ref
            // to code and state.
            //
            // We have to make this a reference to the trait object rather
            // than the trait object itself, because `dyn FnMut` is not sized
            // so we can't compile without the reference.
            let trait_obj: &dyn FnMut(u16, u64) = &closure;
            // Now get a _reference to the reference_, to prepare to coerce
            // to a raw pointer.
            let trait_obj_ref = &trait_obj;
            // Now we want a _pointer_ to the reference to the trait object.
            // That means we want to get a pointer to the `&dyn FnMut`,
            // so we're coercing our `trait_obj_ref` to a `*const c_void`.
            //
            // Note the compiler doesn't guarantee we can cast right to
            // that, so we have to first cast to a `*const _` -- which
            // gets us from reference-land to pointer-land -- and then cast
            // that intermediate type to `*const c_void` -- which is the same
            // as a void pointer in C. In other words, we end up with a
            // pointer to anything we want, hence the wild unsafety of this
            // code and this function as a whole.
            let closure_ptr_ptr = trait_obj_ref as *const _ as *const c_void;
            // Finally, now we have a `*const c_void`, which is a pointer to
            // anything (and demonstrates our complete disregard for the type
            // system!), so we can cast that to our desired type..
            closure_ptr_ptr as *const (u16, u64)
        }

        let mgr = u_sbox.get_mem_mgr_mut();
        let outb_ptr = outb_hdl_as_fn_ptr(outb_hdl.clone());
        mgr.set_outb_address(outb_ptr as u64)?;
        let peb_address = {
            let base_addr = u64::try_from(mgr.shared_mem.base_addr())?;
            mgr.get_peb_address(base_addr)
        }?;
        let page_size = u32::try_from(get_os_page_size())?;
        let seed = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            rng.gen::<u64>()
        };
        unsafe { u_sbox.call_entry_point(RawPtr::from(peb_address), seed, page_size) }?;
        Ok(Sandbox::from(u_sbox))
    }
}

#[cfg(test)]
mod tests {
    use super::evolve_impl;
    use crate::{
        hypervisor::handlers::{MemAccessHandler, OutBHandler},
        testing::{callback_guest_path, simple_guest_path},
        UninitializedSandbox,
    };
    use anyhow::{anyhow, Result};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_evolve() {
        let guest_bin_paths = vec![simple_guest_path().unwrap(), callback_guest_path().unwrap()];
        let outb_arc = {
            let cb: Box<dyn FnMut(u16, u64) -> Result<()>> = Box::new(|_, _| -> Result<()> {
                println!("outb callback in test_evolve");
                Ok(())
            });
            Arc::new(Mutex::new(OutBHandler::from(cb)))
        };
        let mem_access_arc = {
            let cb: Box<dyn FnMut() -> Result<()>> = Box::new(|| -> Result<()> {
                println!("mem access callback in test_evolve");
                Ok(())
            });
            Arc::new(Mutex::new(MemAccessHandler::from(cb)))
        };
        for guest_bin_path in guest_bin_paths {
            let u_sbox = UninitializedSandbox::new(guest_bin_path.clone(), None, None).unwrap();
            evolve_impl(u_sbox, outb_arc.clone(), mem_access_arc.clone())
                .map_err(|e| {
                    anyhow!("error evolving sandbox with guest binary {guest_bin_path}: {e:?}")
                })
                .unwrap();
        }
    }

    #[test]
    fn test_evolve_in_proc() {
        use crate::SandboxRunOptions;

        let guest_bin_paths = vec![simple_guest_path().unwrap(), callback_guest_path().unwrap()];
        let outb_arc = {
            let cb: Box<dyn FnMut(u16, u64) -> Result<()>> = Box::new(|_, _| Ok(()));
            Arc::new(Mutex::new(OutBHandler::from(cb)))
        };
        let mem_access_arc = {
            let cb: Box<dyn FnMut() -> Result<()>> = Box::new(|| Ok(()));
            Arc::new(Mutex::new(MemAccessHandler::from(cb)))
        };
        for guest_bin_path in guest_bin_paths {
            let u_sbox: UninitializedSandbox<'_> = UninitializedSandbox::new(
                guest_bin_path.clone(),
                None,
                Some(SandboxRunOptions::RUN_IN_PROCESS),
            )
            .unwrap();
            #[cfg(target_os = "windows")]
            {
                let err = format!("error evolving sandbox with guest binary {guest_bin_path}");
                let err_str = err.as_str();
                evolve_impl(u_sbox, outb_arc.clone(), mem_access_arc.clone()).expect(err_str);
            }
            #[cfg(target_os = "linux")]
            {
                let res = evolve_impl(u_sbox, outb_arc.clone(), mem_access_arc.clone());
                assert!(res.is_err());
            }
        }
    }
}
