global using NativeHandle = System.UInt64;
using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;
namespace Hyperlight.Wrapper
{
    public class Handle : IDisposable
    {
        /// <summary>
        /// The NativeHandle that represents nothing. This is 
        /// different than a NativeHandle that represents
        /// an empty value, and is different from a NativeHandle
        /// that represents an error. It is indicative of a 
        /// NativeHandle that has not yet been assigned and thus,
        /// does not exist.
        /// </summary>
        public static readonly NativeHandle Zero; // default value is 0

        public Context ctx { get; private set; }
        public NativeHandle handle { get; private set; }
        private bool disposed;

        public Handle(Context ctx, NativeHandle hdl)
        {
            ArgumentNullException.ThrowIfNull(ctx);
            if (Zero == hdl)
            {
                throw new HyperlightException(
                    "Handle wrapper created with empty handle"
                );
            }

            this.ctx = ctx;
            this.handle = hdl;
        }

        /// <summary>
        /// Create a new handle wrapper that references an error
        /// </summary>
        /// <param name="ctx">
        /// the context in which to create the error
        /// </param>
        /// <param name="errMsg">
        /// the message that the error should have
        /// </param>
        /// <returns>
        /// a new wrapper for a new handle that references the new error
        /// </returns>
        public static Handle NewError(Context ctx, String errMsg)
        {
            ArgumentNullException.ThrowIfNull(ctx);
            unsafe
            {
                fixed (char* charPtr = errMsg)
                {
                    var raw = handle_new_err(ctx.ctx, new IntPtr(charPtr));
                    return new Handle(ctx, raw);
                }
            }
        }

        public static Handle NewInt32(Context ctx, int val)
        {
            ArgumentNullException.ThrowIfNull(ctx);
            var rawHdl = int_32_new(ctx.ctx, val);
            return new Handle(ctx, rawHdl);
        }

        public static Handle NewInt64(Context ctx, long val)
        {
            ArgumentNullException.ThrowIfNull(ctx);
            var rawHdl = int_64_new(ctx.ctx, val);
            return new Handle(ctx, rawHdl);
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
                    this.handle = Zero;
                }
                disposed = true;
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

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle handle_new_err(
            NativeContext ctx,
            IntPtr errMsg
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle int_32_new(
                    NativeContext ctx,
                    int val
                );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle int_64_new(
                    NativeContext ctx,
                    long val
                );

#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1401 // P/Invoke method should not be visible
#pragma warning restore CA1707

    }
}
