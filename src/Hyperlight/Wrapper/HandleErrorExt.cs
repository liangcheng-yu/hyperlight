using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public static class NativeHandleWrapperErrorExtensions
    {
        public static void ThrowIfError(this Handle hdl)
        {
            HyperlightException.ThrowIfNull(hdl, Sandbox.CorrelationId.Value!, MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            if (hdl.IsError())
            {
                var errMsg = GetErrorMessage(hdl);
                HyperlightException.LogAndThrowException(
                    errMsg,
                    Sandbox.CorrelationId.Value!,
                    MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            }
        }

        public static String GetErrorMessage(
            this Handle hdl,
            String defaultErrMsg = "Handle was an error but failed to get error message"
        )
        {
            ArgumentNullException.ThrowIfNull(hdl);
            if (!hdl.IsError())
            {
                HyperlightException.LogAndThrowException(
                    "attempted to get error string of a non-error Handle",
                    Sandbox.CorrelationId.Value!,
                   MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            }
            var msgPtr = handle_get_error_message(hdl.ctx.ctx, hdl.handle);
            var result = Marshal.PtrToStringAnsi(msgPtr) ?? string.Empty;
            error_message_free(msgPtr);
            return result;
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true, CharSet = CharSet.Unicode)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern IntPtr handle_get_error_message(
            NativeContext ctx,
            NativeHandle handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern void error_message_free(
            IntPtr errMsg
        );
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707

    }
}
