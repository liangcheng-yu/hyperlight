using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public static class NativeHandleWrapperUIntExtensions
    {
        public static bool IsUInt64(this Handle hdl)
        {
            HyperlightException.ThrowIfNull(hdl, MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            return handle_is_uint_64(hdl.ctx.ctx, hdl.handle);
        }
        public static ulong GetUInt64(this Handle hdl)
        {
            HyperlightException.ThrowIfNull(hdl, MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            if (!hdl.IsUInt64())
            {
                HyperlightException.LogAndThrowException("Handle is not a uint64", MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            }
            return handle_get_uint_64(hdl.ctx.ctx, hdl.handle);
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory


        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool handle_is_uint_64(NativeContext ctx, NativeHandle hdl);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong handle_get_uint_64(NativeContext ctx, NativeHandle hdl);

#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707
    }
}
