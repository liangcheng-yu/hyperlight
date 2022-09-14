using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public static class NativeHandleWrapperIntExtensions
    {
        public static bool IsInt64(this Handle hdl)
        {
            if (hdl == null)
            {
                throw new ArgumentNullException(nameof(hdl));
            }
            return handle_is_int_64(hdl.ctx.ctx, hdl.handle);
        }
        public static long GetInt64(this Handle hdl)
        {
            ArgumentNullException.ThrowIfNull(hdl);
            if (!hdl.IsInt64())
            {
                throw new HyperlightException("Handle is not an int64");
            }
            return handle_get_int_64(hdl.ctx.ctx, hdl.handle);
        }

        public static bool IsInt32(this Handle hdl)
        {
            ArgumentNullException.ThrowIfNull(hdl);
            return handle_is_int_32(hdl.ctx.ctx, hdl.handle);
        }
        public static int GetInt32(this Handle hdl)
        {
            ArgumentNullException.ThrowIfNull(hdl);
            if (!hdl.IsInt32())
            {
                throw new HyperlightException("Handle is not an int32");
            }
            return handle_get_int_32(hdl.ctx.ctx, hdl.handle);
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool handle_is_int_32(NativeContext ctx, NativeHandle hdl);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern int handle_get_int_32(NativeContext ctx, NativeHandle hdl);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool handle_is_int_64(NativeContext ctx, NativeHandle hdl);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long handle_get_int_64(NativeContext ctx, NativeHandle hdl);

#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707
    }
}
