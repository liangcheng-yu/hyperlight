using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Native
{
    // TODO: should merge with or subclass from NativeWrapper

    static class LinuxHyperV
    {
        public static bool IsHypervisorPresent()
        {
            return is_hyperv_linux_present();
        }

        //TODO: investigate why we cannot use SafeDirectory here
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        private static extern bool is_hyperv_linux_present();
#pragma warning restore CA5393
    }
}
