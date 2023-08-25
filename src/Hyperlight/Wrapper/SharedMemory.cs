using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    internal class SharedMemory : IDisposable
    {
        private readonly ulong size;
        internal ulong Size => size;
        private readonly Context ctxWrapper;
        internal Handle handleWrapper { get; private set; }
        private bool disposed;
        internal IntPtr Address
        {
            get
            {
                var addr = shared_memory_get_address(
                    this.ctxWrapper.ctx,
                    this.handleWrapper.handle
                );
                return (IntPtr)addr;
            }
        }

        /// <summary>
        /// Create a new wrapper class for an existing SharedMemory handle
        /// </summary>
        /// <param name="ctx">
        /// the context inside which the SharedMemory is stored
        /// </param>
        /// <param name="getterFn">
        /// the function to get the raw Handle to the SharedMemory
        /// </param>
        /// <exception cref="HyperlightException">
        /// if any given parameters are null, or there was an error creating
        /// the new SharedMemory
        /// </exception>
        internal SharedMemory(
            Context ctx,
            Func<Context, NativeHandle> getterFn
        )
        {

            HyperlightException.ThrowIfNull(
                ctx,
                nameof(ctx),
                GetType().Name
            );
            HyperlightException.ThrowIfNull(
                getterFn,
                nameof(getterFn),
                GetType().Name
            );
            this.ctxWrapper = ctx;
            this.handleWrapper = new Handle(ctx, getterFn(ctx), true);
            var sizeRawHdl = shared_memory_get_size(ctx.ctx, this.handleWrapper.handle);
            using var sizeHdl = new Handle(ctx, sizeRawHdl, true);
            if (!sizeHdl.IsUInt64())
            {
                throw new HyperlightException(
                    "couldn't get the size of given shared memory handle"
                );
            }
            var size = sizeHdl.GetUInt64();
            this.size = size;
        }

        public void Dispose()
        {
            this.Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!this.disposed)
            {
                if (disposing)
                {
                    this.handleWrapper.Dispose();
                }
                this.disposed = true;
            }
        }
#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA1401 // P/Invoke method should not be visible
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory


        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong shared_memory_get_address(
            NativeContext ctx,
            NativeHandle hdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong shared_memory_get_size(
            NativeContext ctx,
            NativeHandle hdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern NativeHandle shared_memory_copy_from_byte_array(
            NativeContext ctx,
            NativeHandle shared_memory_handle,
            NativeHandle byte_array_handle,
            ulong address,
            ulong arr_start,
            ulong arr_length
        );
    }

#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning disable CA1401 // P/Invoke method should not be visible
#pragma warning restore CA1707
}
