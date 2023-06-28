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
            IntPtr sourceAddress,
            ulong pml4_addr,
            ulong size,
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
            var addrs = new HypervisorAddrs(
                entryPoint,
                (ulong)sourceAddress.ToInt64(),
                size
            );
            var rawHdl = hyperv_linux_create_driver(
                ctxWrapper.ctx,
                addrs,
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
            hdlWrapper.ThrowIfError();
        }

        internal override void ResetRSP(ulong rsp)
        {
            var rawHdl = hyperv_linux_set_rsp(
                this.ctxWrapper.ctx,
                this.driverHdlWrapper.handle,
                rsp
            );
            // marking 'false' here to indicate the handle wrapper should
            // not automatically throw if the raw handle is an error,
            // so we can throw our custom HyperVOnLinuxException
            // here
            using var hdl = new Handle(this.ctxWrapper, rawHdl, false);
            if (hdl.IsError())
            {
                HyperlightException.LogAndThrowException<HyperVOnLinuxException>(
                    $"Failed setting RSP Error: {hdl.GetErrorMessage()}",
                    GetType().Name
                );
            }
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
            hdlWrapper.ThrowIfError();
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

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle hyperv_linux_create_driver(
            NativeContext ctx,
            HypervisorAddrs addrs,
            ulong rsp,
            ulong pml4
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle hyperv_linux_dispatch_call_from_host(
            NativeContext ctx,
            NativeHandle driver_hdl,
            NativeHandle outb_handle_fn_hdl,
            NativeHandle mem_access_fn_hdl,
            ulong dispatch_func_addr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
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

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle hyperv_linux_set_rsp(
            NativeContext ctx,
            NativeHandle driverHdl,
            ulong rspVal
        );

#pragma warning restore CA5393

    }
}
