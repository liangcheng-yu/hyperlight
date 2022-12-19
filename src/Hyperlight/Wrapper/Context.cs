global using NativeContext = System.IntPtr;
using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;
namespace Hyperlight.Wrapper
{
    public class Context : IDisposable
    {
        /// <summary>
        /// Create a new context wrapper with a newly allocated
        /// context.
        /// </summary>
        public Context(string corrID) : this(context_new(), corrID)
        {
        }

        public NativeContext ctx { get; private set; }
        private bool disposed;
        private Context(NativeContext ctx, string corrID)
        {
            if (ctx == IntPtr.Zero)
            {
                HyperlightException.LogAndThrowException<HyperlightException>("Invalid, empty context passed to Context constructor", corrID, GetType().Name);
            }

            this.ctx = ctx;
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
                    context_free(this.ctx);
                }
                this.disposed = true;
            }
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA1401 // P/Invoke method should not be visible
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeContext context_new();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern void context_free(NativeContext context);
#pragma warning restore CA5393
#pragma warning restore CA1401
#pragma warning restore CA1707
    }
}
