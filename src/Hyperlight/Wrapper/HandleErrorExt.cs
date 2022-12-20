using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public static class NativeHandleWrapperErrorExtensions
    {
        /// Extension method on Handle to throw if hdl was an error
        public static void ThrowIfError(this Handle hdl)
        {
            HyperlightException.ThrowIfNull(
                hdl,
                MethodBase.GetCurrentMethod()!.DeclaringType!.Name
            );
            ThrowIfError(hdl.ctx, hdl.handle);
        }

        /// <summary>
        /// Given a context wrapper and a raw handle, throw if the handle represents
        /// an error
        /// </summary>
        /// <param name="ctx">the context wrapper</param>
        /// <param name="hdl">the raw handle in question</param>
        public static void ThrowIfError(Context ctx, NativeHandle hdl)
        {
            HyperlightException.ThrowIfNull(
                ctx,
                MethodBase.GetCurrentMethod()!.DeclaringType!.Name
            );
            if (Handle.IsError(hdl))
            {
                {
                    var errMsg = GetErrorMessage(ctx, hdl);
                    HyperlightException.LogAndThrowException(
                        errMsg,
                        MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
                }
            }
        }

        /// <summary>
        /// Get the error message from the hdl, or return defaultErrMsg
        /// if no error message could be fetched or the handle was not
        /// an error
        /// </summary>
        /// <param name="hdl">
        /// the handle from which to get the error message
        /// </param>
        /// <param name="defaultErrMsg">
        /// the message to return if the error message couldn't be fetched
        /// for any reason
        /// </param>
        /// <returns></returns>
        public static String GetErrorMessage(
            this Handle hdl,
            String defaultErrMsg = "Handle was an error but failed to get error message"
        )
        {
            HyperlightException.ThrowIfNull(
                hdl,
                nameof(hdl),
                "GetErrorMessage"
            );
            return GetErrorMessage(hdl.ctx, hdl.handle, defaultErrMsg);
        }
        private static String GetErrorMessage(
            Context ctx,
            NativeHandle hdl,
            String defaultErrMsg = "Handle was an error but failed to get error message"
        )
        {
            HyperlightException.ThrowIfNull(
                hdl,
                nameof(hdl),
                "GetErrorMessage"
            );
            if (!Handle.IsError(hdl))
            {
                HyperlightException.LogAndThrowException(
                    "attempted to get error string of a non-error Handle",
                    MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            }
            var msgPtr = handle_get_error_message(ctx.ctx, hdl);
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
