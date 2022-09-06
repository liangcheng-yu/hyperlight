global using NativeHandle = System.UInt64;
using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;
namespace Hyperlight.Wrapper
{
    public class Handle : IDisposable
    {
        public Context ctx { get; private set; }
        public NativeHandle handle { get; private set; }
        private bool disposed;

        public Handle(Context ctx, NativeHandle hdl)
        {
            this.ctx = ctx;
            this.handle = hdl;
        }

        public void Dispose()
        {
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!this.disposed)
            {
                if (disposing)
                {
                    handle_free(this.ctx.ctx, this.handle);
                }
                disposed = true;
            }
        }

        public static void ThrowIfNull(Handle hdl)
        {
            if (null == hdl)
            {
                throw new ArgumentNullException(nameof(hdl));
            }
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA1401 // P/Invoke method should not be visible
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern bool handle_free(
            NativeContext context,
            NativeHandle handle
        );

#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1401 // P/Invoke method should not be visible
#pragma warning restore CA1707

    }
}
