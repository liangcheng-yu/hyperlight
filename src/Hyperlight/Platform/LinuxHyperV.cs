using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Native
{
    // TODO: should merge with or subclass from NativeWrapper

    static class LinuxHyperV
    {
        // TODO: This should come from configuration.
        public const bool REQUIRE_STABLE_API = false;

        public static bool IsHypervisorPresent()
        {
            return is_hyperv_linux_present(LinuxHyperV.REQUIRE_STABLE_API);
        }

        //TODO: investigate why we cannot use SafeDirectory here
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool is_hyperv_linux_present(
            [MarshalAs(UnmanagedType.U1)] bool require_stable_api
        );
#pragma warning restore CA5393
    }
}
