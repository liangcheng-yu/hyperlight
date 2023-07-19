using System;
using System.Reflection;

using System.Runtime.InteropServices;
using Hyperlight.Core;
using Microsoft.Extensions.Configuration;

namespace Hyperlight.Wrapper
{
    internal static class SandboxMemoryLayout
    {
        static readonly ulong pageTableSize = mem_layout_get_page_table_size();
        public static ulong PML4Offset = mem_layout_get_pml4_offset();
        public static ulong PDOffset = mem_layout_get_pd_offset();
        public static ulong PDPTOffset = mem_layout_get_pdpt_offset();
        public static ulong CodeOffSet = pageTableSize;
        public static ulong BaseAddress = mem_layout_get_base_address();
        public static ulong PML4GuestAddress = BaseAddress;
        public static ulong PDPTGuestAddress = BaseAddress + PDPTOffset;
        public static ulong PDGuestAddress = BaseAddress + PDOffset;
        public static ulong GuestCodeAddress = BaseAddress + CodeOffSet;

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_page_table_size();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_base_address();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_pml4_offset();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_pd_offset();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_pdpt_offset();

#pragma warning restore CA5393
#pragma warning restore CA1707
    }
}
