using System;
using System.Diagnostics;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Core;
using Hyperlight.Native;
using Hyperlight.Wrapper;

namespace Hyperlight.Hypervisors;

internal sealed class KVM : Hypervisor, IDisposable
{
    private bool disposedValue;
    readonly Context ctxWrapper;
    readonly Handle driverHdlWrapper;
    readonly OutbHandler outbFnHdlWrapper;
    readonly MemAccessHandler memAccessFnHdlWrapper;

    internal KVM(
        Context ctx,
        Handle mgr_hdl,
        IntPtr sourceAddr,
        ulong pml4Addr,
        ulong entryPoint,
        ulong rsp,
        Action<ushort, byte> outb,
        Action handleMemoryAccess
    ) : base(
        sourceAddr,
        entryPoint,
        rsp,
        outb,
        handleMemoryAccess
    )
    {
        HyperlightException.ThrowIfNull(
            ctx,
            nameof(ctx),
            GetType().Name
        );

        var rawHdl = kvm_create_driver(
            ctx.ctx,
            mgr_hdl.handle,
            pml4Addr,
            entryPoint,
            rsp
        );
        this.ctxWrapper = ctx;
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
        var rawHdl = kvm_dispatch_call_from_host(
            this.ctxWrapper.ctx,
            this.driverHdlWrapper.handle,
            this.outbFnHdlWrapper.handle,
            this.memAccessFnHdlWrapper.handle,
            pDispatchFunction
        );
        using var hdlWrapper = new Handle(this.ctxWrapper, rawHdl, true);
    }

    internal override void ResetRSP(ulong rsp)
    {
        // the driver resets it internally every time a guest function is dispatched
    }

    internal override void Initialise(IntPtr pebAddress, ulong seed, uint pageSize)
    {
        var rawHdl = kvm_initialise(
            this.ctxWrapper.ctx,
            this.driverHdlWrapper.handle,
            this.outbFnHdlWrapper.handle,
            this.memAccessFnHdlWrapper.handle,
            (ulong)pebAddress.ToInt64(),
            seed,
            pageSize
        );
        using var hdlWrapper = new Handle(this.ctxWrapper, rawHdl, true);
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

    ~KVM()
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
    private static extern NativeHandle kvm_create_driver(
        NativeContext ctx,
        NativeHandle mgr_hdl,
        ulong pml4Addr,
        ulong entryPoint,
        ulong rsp
    );

    [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
    private static extern NativeHandle kvm_dispatch_call_from_host(
        NativeContext ctx,
        NativeHandle driver_hdl,
        NativeHandle outb_handle_fn_hdl,
        NativeHandle mem_access_fn_hdl,
        ulong dispatch_func_addr
    );

    [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
    private static extern NativeHandle kvm_initialise(
        NativeContext ctx,
        NativeHandle driver_hdl,
        NativeHandle outb_handle_fn_hdl,
        NativeHandle mem_access_fn_hdl,
        ulong peb_addr,
        ulong seed,
        uint pageSize
    );

#pragma warning restore CA5393
}
