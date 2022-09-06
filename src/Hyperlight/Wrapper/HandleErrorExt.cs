using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public static class NativeHandleWrapperErrorExtensions
    {
        public static void ThrowIfError(this Handle hdl)
        {
            ArgumentNullException.ThrowIfNull(hdl);
            if (handle_is_error(hdl.handle))
            {
                var errorMessage = handle_get_error_message(hdl.ctx.ctx, hdl.handle);
                throw new HyperlightException(errorMessage);
            }
        }


#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern bool handle_is_error(NativeHandle handle);

        [return: MarshalAs(UnmanagedType.LPWStr)]
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true, CharSet = CharSet.Unicode)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern string handle_get_error_message(
            NativeContext ctx,
            NativeHandle handle
        );
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707

    }
}
