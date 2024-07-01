using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;
using Hyperlight.Native;
using Hyperlight.Wrapper;

namespace Hyperlight.Hypervisors
{
    internal sealed class HyperVOnLinux : Hypervisor, IDisposable
    {
        private bool disposedValue;
        readonly Context ctxWrapper;
        readonly Handle driverHdlWrapper;
        readonly OutbHandler outbFnHdlWrapper;
        readonly MemAccessHandler memAccessFnHdlWrapper;

        internal HyperVOnLinux(
            Context ctxWrapper,
            Handle mgr_hdl,
            IntPtr sourceAddress,
            ulong pml4_addr,
            ulong entryPoint,
            ulong rsp,
            Action<ushort, byte> outb,
            Action handleMemoryAccess
        ) : base(
            sourceAddress,
            entryPoint,
            rsp,
            outb,
            handleMemoryAccess
        )
        {
            var rawHdl = hyperv_linux_create_driver(
                ctxWrapper.ctx,
                mgr_hdl.handle,
                entryPoint,
                rsp,
                pml4_addr
            );
            this.ctxWrapper = ctxWrapper;
            this.driverHdlWrapper = new Handle(ctxWrapper, rawHdl, true);
            this.outbFnHdlWrapper = new OutbHandler(
                this.ctxWrapper,
                outb
            );
            this.memAccessFnHdlWrapper = new MemAccessHandler(
                this.ctxWrapper,
                handleMemoryAccess
            );
        }

        internal override void DispatchCallFromHost(ulong pDispatchFunction)
        {
            using var hdlWrapper = new Handle(
                this.ctxWrapper,
                hyperv_linux_dispatch_call_from_host(
                    this.ctxWrapper.ctx,
                    this.driverHdlWrapper.handle,
                    this.outbFnHdlWrapper.handle,
                    this.memAccessFnHdlWrapper.handle,
                    pDispatchFunction
                )
            );
            hdlWrapper.ThrowIfUnusable();
        }

        internal override void ResetRSP(ulong rsp)
        {
            // the driver resets it internally every time a guest function is dispatched
        }

        internal override void Initialise(IntPtr pebAddress, ulong seed, uint pageSize)
        {
            using var hdlWrapper = new Handle(
                this.ctxWrapper,
                hyperv_linux_initialise(
                    this.ctxWrapper.ctx,
                    this.driverHdlWrapper.handle,
                    this.outbFnHdlWrapper.handle,
                    this.memAccessFnHdlWrapper.handle,
                    (ulong)pebAddress.ToInt64(),
                    seed,
                    pageSize
                )
            );
            hdlWrapper.ThrowIfUnusable();
        }

        private void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    // dispose managed state (managed objects)
                    this.driverHdlWrapper.Dispose();
                    this.outbFnHdlWrapper.Dispose();
                    this.memAccessFnHdlWrapper.Dispose();
                }
                disposedValue = true;
            }
        }

        ~HyperVOnLinux()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: false);
        }

        public override void Dispose()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

        //TODO: investigate why we cannot use SafeDirectory here
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle hyperv_linux_create_driver(
            NativeContext ctx,
            NativeHandle mgr_hdl,
            ulong entrypoint,
            ulong rsp,
            ulong pml4
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle hyperv_linux_dispatch_call_from_host(
            NativeContext ctx,
            NativeHandle driver_hdl,
            NativeHandle outb_handle_fn_hdl,
            NativeHandle mem_access_fn_hdl,
            ulong dispatch_func_addr
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle hyperv_linux_initialise(
            NativeContext ctx,
            NativeHandle driver_hdl,
            NativeHandle outb_handle_fn_hdl,
            NativeHandle mem_access_fn_hdl,
            ulong peb_addr,
            ulong seed,
            uint page_size
        );

#pragma warning restore CA5393

    }
}
