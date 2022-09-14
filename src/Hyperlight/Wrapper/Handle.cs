global using NativeHandle = System.UInt64;
using System;
using System.Text;
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

        /// <summary>
        /// Get the context which stores the value backing the NativeHandle
        /// that this Handle wraps
        /// </summary>
        public Context ctx { get; private set; }

        /// <summary>
        /// Get the NativeHandle that is wrapped by this Handle
        /// </summary>
        public NativeHandle handle { get; private set; }
        private bool disposed;

        /// <summary>
        /// Create a new wrapper around hdl.
        /// </summary>
        /// <param name="ctx">
        /// The context in which hdl is stored. If you don't pass
        /// the same context that holds memory that hdl references,
        /// undefined behavior will result
        /// </param>
        /// <param name="hdl">the NativeHandle to wrap</param>
        /// <exception cref="HyperlightException">
        /// If hdl is equal to Handle.Zero
        /// </exception>
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
        /// Create a wrapper for a handle that references an error
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
            var barr = Encoding.Default.GetBytes(errMsg);
            Array.Resize(ref barr, barr.Length + 1);
            barr[barr.Length - 1] = (byte)'\0';
            var raw = handle_new_err(ctx.ctx, barr);
            return new Handle(ctx, raw);
        }

        /// <summary>
        /// Create a new NativeHandle in ctx with the given
        /// value val, then return a new Handle that wraps that 
        /// NativeHandle
        /// </summary>
        /// <param name="ctx">
        /// the context in which to store the value
        /// </param>
        /// <param name="val">
        /// the value for which a handle should be created
        /// </param>
        /// <returns>
        /// a Handle that wraps the NativeHandle that references
        /// the value
        /// </returns>
        public static Handle NewInt32(Context ctx, int val)
        {
            ArgumentNullException.ThrowIfNull(ctx);
            var rawHdl = int_32_new(ctx.ctx, val);
            return new Handle(ctx, rawHdl);
        }

        /// <summary>
        /// Create a new NativeHandle in ctx with the given
        /// value val, then return a new Handle that wraps that 
        /// NativeHandle
        /// </summary>
        /// <param name="ctx">
        /// the context in which to store the value
        /// </param>
        /// <param name="val">
        /// the value for which a handle should be created
        /// </param>
        /// <returns>
        /// a Handle that wraps the NativeHandle that references
        /// the value
        /// </returns>
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

        /// <summary>
        /// Determine whether this is an empty handle.
        /// 
        /// Note that this is not the same thing as Handle.Zero.
        /// See the documentation for Handle.Zero (above) for more
        /// detail.
        /// </summary>
        /// <returns>true if this handle is empty, false otherwise</returns>
        public bool IsEmpty()
        {
            return handle_is_empty(this.handle);
        }

        public bool IsInvalid()
        {
            return handle_is_invalid(this.handle);
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA1401 // P/Invoke method should not be visible
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        public static extern bool handle_free(
            NativeContext context,
            NativeHandle handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool handle_is_empty(NativeHandle handle);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool handle_is_invalid(NativeHandle handle);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle handle_new_err(
            NativeContext ctx,
            [In] byte[] errMsg
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
