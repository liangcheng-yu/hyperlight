global using NativeContext = System.IntPtr;
global using NativeHandle = System.UInt64;

using System;
using System.ComponentModel.DataAnnotations;
using System.Runtime.InteropServices;
namespace Hyperlight.Core
{
    public static class NativeWrapper
    {
#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA1401 // P/Invoke method should not be visible
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern NativeContext context_new();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern void context_free(NativeContext context);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern bool handle_free(
            NativeContext context,
            NativeHandle handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern bool handle_is_error(NativeHandle handle);

        [return: MarshalAs(UnmanagedType.LPWStr)]
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true, CharSet = CharSet.Unicode)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern string handle_get_error_message(
            NativeContext ctx,
            NativeHandle handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern NativeHandle mem_config_new(
            NativeContext context,
            ulong input_data_size,
            ulong output_data_size,
            ulong function_definition_size,
            ulong host_exception_size,
            ulong guest_error_message_size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong mem_config_get_guest_error_message_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern void mem_config_set_guest_error_message_size(
            NativeContext context,
            NativeHandle handle,
            ulong size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong mem_config_get_host_function_definition_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern void mem_config_set_host_function_definition_size(
            NativeContext context,
            NativeHandle handle,
            ulong size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong mem_config_get_host_exception_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong mem_config_set_host_exception_size(
            NativeContext context,
            NativeHandle mem_config_handle,
            ulong size
        );


        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong mem_config_get_input_data_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong mem_config_set_input_data_size(
            NativeContext context,
            NativeHandle mem_config_handle,
            ulong size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong mem_config_get_output_data_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern ulong mem_config_set_output_data_size(
            NativeContext context,
            NativeHandle mem_config_handle,
            ulong size
        );
#pragma warning restore CA5393
#pragma warning restore CA1401
#pragma warning restore CA1707

        public static void ThrowIfHandleIsError(
            NativeContext context,
            NativeHandle handle
        )
        {
            if (handle_is_error(handle))
            {
                var errorMessage = handle_get_error_message(context, handle);
                throw new HyperlightException(errorMessage);
            }
        }

    }
}
