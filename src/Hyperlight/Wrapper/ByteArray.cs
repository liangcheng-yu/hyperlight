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

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory


        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern unsafe NativeHandle byte_array_new(
            NativeContext ctx,
            byte* arr_ptr,
            ulong arr_len
        );

#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707
    }
}
