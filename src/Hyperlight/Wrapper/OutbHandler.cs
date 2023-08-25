using System;
using System.Runtime.InteropServices;
using Hyperlight;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    [StructLayout(LayoutKind.Sequential)]
    readonly struct OutbHandlerCallback
    {
        readonly IntPtr func;

        OutbHandlerCallback(IntPtr func)
        {
            this.func = func;
        }
    }

    public sealed class OutbHandler : IDisposable
    {
        private bool disposedValue;
        private readonly HandleWithGCHandle hpr;
        public NativeHandle handle => hpr.handle;

        private delegate void CallOutbDelegate(ushort port, byte value);

        public OutbHandler(Context ctx, Action<ushort, byte> handlerFn)
        {
            HyperlightException.ThrowIfNull(
                ctx,
                nameof(ctx),
                GetType().Name
            );
            HyperlightException.ThrowIfNull(
                handlerFn,
                nameof(handlerFn),
                GetType().Name
            );
            // we need to wrap our handleFn -- which is an Action<T1, T2>
            // in a non-generic type, so that we can GCHandle.Alloc it
            // and subsequently get a function pointer to it.

            // Handle references func, so we do need to alloc it --
            // which tells the GC that we will free the memory and it 
            // should not -- but we do not need to pin it -- which adds
            // additional functionality on top of the alloc behavior
            // that we don't need.
            //
            // The following blog post has a fairly practical explanation of
            // these concepts. Scroll to the paragraph that starts with 
            // "Finally, a word on pinning"
            //
            // https://review.learn.microsoft.com/en-us/archive/blogs/cbrumme/asynchronous-operations-pinning
            // 
            // For further reading, see the following MS documentation
            // on pinning:
            // https://learn.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.gchandletype?view=net-7.0

            CallOutbDelegate wrapper = (port, payload) =>
            {
                handlerFn(port, payload);
            };

            GCHandle gcHdl = GCHandle.Alloc(wrapper);
            IntPtr fnPtr = Marshal.GetFunctionPointerForDelegate<CallOutbDelegate>(
                wrapper
            );



            this.hpr = new HandleWithGCHandle(
                gcHdl,
                new Handle(
                    ctx,
                    outb_fn_handler_create(
                        ctx.ctx,
                        fnPtr
                    )
                )
            );
        }

        public void Call(ushort port, byte payload)
        {
            var rawHdl = outb_fn_handler_call(this.hpr.ctx.ctx, this.hpr.handle, port, payload);
            using (var hdl = new Handle(this.hpr.ctx, rawHdl))
            {
                hdl.ThrowIfError();
            }
        }

        private void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    this.hpr.Dispose();
                }
                // if we have unmanaged memory, free it here
                disposedValue = true;
            }
        }

        public void Dispose()
        {
            this.Dispose(true);
            GC.SuppressFinalize(this);
        }

        ~OutbHandler()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: false);
        }



#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle outb_fn_handler_create(
            NativeContext ctx,
            IntPtr outb_fn_ptr
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle outb_fn_handler_call(
            NativeContext ctx,
            NativeHandle outb_fn_handler_ref,
            ushort port,
            byte payload
        );
#pragma warning restore CA5393
    }
}
