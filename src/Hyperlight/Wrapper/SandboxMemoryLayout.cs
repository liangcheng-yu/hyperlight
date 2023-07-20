using System;
using System.Reflection;

using System.Runtime.InteropServices;
using Hyperlight.Core;
using Microsoft.Extensions.Configuration;

namespace Hyperlight.Wrapper
{
    internal static class SandboxMemoryLayout
    {
        public static ulong BaseAddress = mem_layout_get_base_address();
        public static ulong PML4GuestAddress = BaseAddress;

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_base_address();

#pragma warning restore CA5393
#pragma warning restore CA1707
    }
}
