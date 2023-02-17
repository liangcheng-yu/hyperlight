using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public static class NativeHandleWrapperBooleanExtensions
    {
        public static bool IsBoolean(this Handle hdl)
        {
            HyperlightException.ThrowIfNull(hdl, MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            return handle_is_boolean(hdl.ctx.ctx, hdl.handle);
        }
        public static bool GetBoolean(this Handle hdl)
        {
            HyperlightException.ThrowIfNull(hdl, MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            return handle_get_boolean(hdl.ctx.ctx, hdl.handle);
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool handle_is_boolean(NativeContext ctx, NativeHandle hdl);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool handle_get_boolean(NativeContext ctx, NativeHandle hdl);
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707
    }
}
