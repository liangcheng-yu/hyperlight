using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Google.FlatBuffers;
using Hyperlight.Generated;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public static class NativeHandleWrapperGuestErrorExtensions
    {
        public static bool IsGuestError(this Handle hdl)
        {
            HyperlightException.ThrowIfNull(hdl, MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            return handle_is_guest_error(hdl.ctx.ctx, hdl.handle);
        }
        public static GuestError GetGuestError(this Handle hdl)
        {
            HyperlightException.ThrowIfNull(hdl, MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            var bufferPtr = handle_get_guest_error_flatbuffer(hdl.ctx.ctx, hdl.handle);
            if (bufferPtr == IntPtr.Zero)
            {
                // TODO implement get last error to get the error message 
                throw new HyperlightException("Failed to get guest error");
            }

            // TODO: This is inefficient, need to look at implementing ByteBufferAllocator or using ENABLE_SPAN_T and UNSAFE_BYTEBUFFER
            // see https://google.github.io/flatbuffers/flatbuffers_guide_use_c-sharp.html
            // The buffer returned is a byte buffer with a size prefix in the first 4 bytes
            // we read that and then add its length to the value returned to get the total length of the array we need to allocate

            var bufferSize = GetBufferSize(bufferPtr);
            var data = new byte[bufferSize];
            Marshal.Copy(bufferPtr, data, 0, (int)bufferSize);
            var byteBufferWithSize = new ByteBuffer(data);
            var byteBuffer = ByteBufferUtil.RemoveSizePrefix(byteBufferWithSize);
            var guestError = GuestError.GetRootAsGuestError(byteBuffer);
            guest_error_flatbuffer_free(bufferPtr);
            return guestError;

        }
        static unsafe uint GetBufferSize(IntPtr buffer)
        {
            // The additional 4 bytes is added as the value in the length field does not include itself
            var sizePrefix = BitConverter.IsLittleEndian ? *(uint*)buffer : ByteBuffer.ReverseBytes(*(uint*)buffer);
            return sizePrefix + sizeof(uint);
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool handle_is_guest_error(NativeContext ctx, NativeHandle hdl);

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern IntPtr handle_get_guest_error_flatbuffer(NativeContext ctx, NativeHandle hdl);

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool guest_error_flatbuffer_free(IntPtr bufferPtr);
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707

    }
}
