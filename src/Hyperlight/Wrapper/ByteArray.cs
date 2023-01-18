using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public class ByteArray : IDisposable
    {
        private readonly Context ctxWrapper;
        public Handle handleWrapper { get; private set; }
        private bool disposed;

        public ByteArray(
            Context ctx,
            byte[] arr
        )
        {
            HyperlightException.ThrowIfNull(arr, GetType().Name);

            this.ctxWrapper = ctx;
            unsafe
            {
                fixed (byte* arr_ptr = arr)
                {
                    var rawHdl = byte_array_new(
                        ctxWrapper.ctx,
                        arr_ptr,
                        (ulong)arr.Length
                    );
                    this.handleWrapper = new Handle(ctxWrapper, rawHdl);
                    this.handleWrapper.ThrowIfError();
                }
            }

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

        /// Returns a copy of the byte array contents from the context.
        public unsafe byte[] GetContents()
        {
            ulong len = byte_array_len(this.ctxWrapper.ctx, this.handleWrapper.handle);
            byte* arr_ptr = byte_array_get(this.ctxWrapper.ctx, this.handleWrapper.handle);
            if (arr_ptr == null)
            {
                // TODO: How do I get the error from the context and throw it?
                throw new InvalidOperationException("ByteArray was not present in the Context");
            }

            var contents = new byte[len];
            Marshal.Copy(new IntPtr(arr_ptr), contents, 0, contents.Length);
            return contents;
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern unsafe NativeHandle byte_array_new(
            NativeContext ctx,
            byte* arr_ptr,
            ulong arr_len
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle byte_array_len(
            NativeContext ctx,
            NativeHandle bye_array_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern unsafe byte* byte_array_get(
            NativeContext ctx,
            NativeHandle bye_array_handle
        );

#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707
    }
}
