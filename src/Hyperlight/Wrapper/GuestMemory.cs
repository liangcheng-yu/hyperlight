using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public class GuestMemoryWrapper : IDisposable
    {
        private Context ctxWrapper;
        public Handle handleWrapper { get; private set; }
        private bool disposed;
        public IntPtr Address
        {
            get
            {
                var addr = guest_memory_get_address(
                    this.ctxWrapper.ctx,
                    this.handleWrapper.handle
                );
                return new IntPtr((int)addr);
            }
        }
        public GuestMemoryWrapper(
            Context ctxWrapper,
            ulong size
        )
        {
            this.ctxWrapper = ctxWrapper;
            this.handleWrapper = new Handle(
                ctxWrapper,
                guest_memory_new(
                    this.ctxWrapper.ctx,
                    size
                )
            );

        }

        // TODO: make this a long rather than ulong
        public void WriteInt64(
            IntPtr addr,
            ulong val
        )
        {
            var rawHdl = guest_memory_write_int_64(
                this.ctxWrapper.ctx,
                this.handleWrapper.handle,
                (ulong)addr.ToInt64(),
                val
            );
            using (var hdl = new Handle(this.ctxWrapper, rawHdl))
            {
                hdl.ThrowIfError();
            }
        }

        public long ReadInt64(
            IntPtr addr
        )
        {
            var rawHdl = guest_memory_read_int_64(
                this.ctxWrapper.ctx,
                this.handleWrapper.handle,
                (ulong)addr.ToInt64()
            );
            using (var hdl = new Handle(this.ctxWrapper, rawHdl))
            {

                hdl.ThrowIfError();
                return hdl.GetInt64();
            }
        }

        // TODO: implement
        public void WriteInt32(
            IntPtr addr,
            int val
        )
        {
            var hdlRes = guest_memory_write_int_32(
                this.ctxWrapper.ctx,
                this.handleWrapper.handle,
                (ulong)addr.ToInt64(),
                val
            );
            using (var hdlWrapper = new Handle(this.ctxWrapper, hdlRes))
            {
                hdlWrapper.ThrowIfError();
            }
        }

        public int ReadInt32(
            IntPtr addr
        )
        {
            var rawHdl = guest_memory_read_int_32(
                this.ctxWrapper.ctx,
                this.handleWrapper.handle,
                (ulong)addr.ToInt64()
            );
            using (var hdl = new Handle(this.ctxWrapper, rawHdl))
            {
                hdl.ThrowIfError();
                return hdl.GetInt32();
            }
        }

        public void CopyByteArray(
            Context ctxWrapper,
            byte[] arr,
            IntPtr addr
        )
        {
            ByteArray.ThrowIfNull(arr);

            using var barr = new ByteArray(ctxWrapper, arr);
            this.CopyByteArray(
                barr,
                (ulong)addr.ToInt64(),
                0,
                (ulong)arr.Length
            );
        }

        public void CopyByteArray(
            ByteArray arr,
            ulong addr,
            ulong arrStart,
            ulong arrLength
        )
        {
            ByteArray.ThrowIfNull(arr);

            var rawHdl = guest_memory_copy_byte_array(
                this.ctxWrapper.ctx,
                this.handleWrapper.handle,
                arr.handleWrapper.handle,
                addr,
                arrStart,
                arrLength
            );
            using (var hdl = new Handle(this.ctxWrapper, rawHdl))
            {
                hdl.ThrowIfError();
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
#pragma warning disable CA1401 // P/Invoke method should not be visible
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern NativeHandle guest_memory_new(
            NativeContext ctx,
            ulong size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong guest_memory_get_address(
            NativeContext ctx,
            NativeHandle hdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern NativeHandle guest_memory_copy_byte_array(
            NativeContext ctx,
            NativeHandle guest_memory_handle,
            NativeHandle byte_array_handle,
            ulong address,
            ulong arr_start,
            ulong arr_length
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern NativeHandle guest_memory_write_int_64(
            NativeContext ctx,
            NativeHandle guest_memory_handle,
            ulong address,
            ulong val
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern NativeHandle guest_memory_read_int_32(
            NativeContext ctx,
            NativeHandle guest_memory_handle,
            ulong address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern NativeHandle guest_memory_read_int_64(
            NativeContext ctx,
            NativeHandle guest_memory_handle,
            ulong address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern NativeHandle guest_memory_write_int_32(
            NativeContext ctx,
            NativeHandle guest_memory_handle,
            ulong address,
            int val
        );
    }

#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning disable CA1401 // P/Invoke method should not be visible
#pragma warning restore CA1707
}
