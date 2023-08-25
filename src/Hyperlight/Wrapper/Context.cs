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
        public Context(string correlationId)
        {
            var new_ctx = IntPtr.Zero;
            var buffer = System.Text.Encoding.UTF8.GetBytes(correlationId);
            unsafe
            {
                fixed (byte* buffer_ptr = buffer)
                {
                    new_ctx = context_new((IntPtr)buffer_ptr);
                }
            }

            if (new_ctx == IntPtr.Zero)
            {
                HyperlightException.LogAndThrowException<HyperlightException>("Invalid, empty context recieved from context_new call", GetType().Name);
            }

            this.ctx = new_ctx;
        }

        /// <summary>
        /// Gets the current request Correlation ID
        /// </summary>
        public string GetCorrelationId()
        {
            var rawHdl = get_correlation_id(this.ctx);
            using var hdl = new Handle(this, rawHdl, true);
            if (!hdl.IsString())
            {
                throw new HyperlightException("get_correlation_id did not return a string");
            }
            if (hdl.GetString() == null)
            {
                throw new HyperlightException("get_correlation_id returned a null string");
            }
            return hdl.GetString()!;
        }

        /// <summary>
        /// sets the current request Correlation ID
        /// </summary>
        public void SetCorrelationId(string correlationId)
        {
            ulong rawHdl = 0;
            var buffer = System.Text.Encoding.UTF8.GetBytes(correlationId);
            unsafe
            {
                fixed (byte* buffer_ptr = buffer)
                {
                    rawHdl = set_correlation_id(this.ctx, (IntPtr)buffer_ptr);
                }
            }

            using var hdl = new Handle(this, rawHdl, true);
        }

        public NativeContext ctx { get; private set; }
        private bool disposed;

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

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeContext context_new(IntPtr correlationId);

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle get_correlation_id(NativeContext context);

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle set_correlation_id(NativeContext context, IntPtr correlationId);

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern void context_free(NativeContext context);
#pragma warning restore CA5393
#pragma warning restore CA1401
#pragma warning restore CA1707
    }
}
