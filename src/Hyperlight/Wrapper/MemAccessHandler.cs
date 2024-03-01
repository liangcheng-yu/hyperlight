using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public sealed class MemAccessHandler : IDisposable
    {
        private bool disposedValue;
        private readonly HandleWithGCHandle hpr;

        public NativeHandle handle => hpr.handle;
        public MemAccessHandler(Context ctx, Action handlerFn)
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

            GCHandle gcHdl = GCHandle.Alloc(handlerFn);
            IntPtr fnPtr = Marshal.GetFunctionPointerForDelegate(handlerFn);

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
            this.hpr = new HandleWithGCHandle(
                gcHdl,
                new Handle(
                    ctx,
                    mem_access_handler_create(
                        ctx.ctx,
                        fnPtr
                    )
                )
            );
        }

        public void Call()
        {
            var rawHdl = mem_access_handler_call(this.hpr.ctx.ctx, this.hpr.handle);
            using (var hdl = new Handle(this.hpr.ctx, rawHdl))
            {
                hdl.ThrowIfUnusable();
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

        ~MemAccessHandler()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: false);
        }

        //TODO: investigate why we cannot use SafeDirectory here
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_access_handler_create(
            NativeContext ctx,
            IntPtr mem_access_fn_ptr
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_access_handler_call(
            NativeContext ctx,
            NativeHandle mem_access_fn_ref
        );
#pragma warning restore CA5393
    }
}
