using System;
using System.Runtime.InteropServices;

namespace Hyperlight.Native
{
    // TODO: should merge with or subclass from NativeWrapper

    static class LinuxHyperV
    {
        // TODO: This should come from configuration.
        public const bool REQUIRE_STABLE_API = false;

        public const int HV_X64_REGISTER_CR0 = 262144;
        public const int HV_X64_REGISTER_CR3 = 262146;
        public const int HV_X64_REGISTER_CR4 = 262147;
        public const int HV_X64_REGISTER_EFER = 524289;
        public const int HV_X64_REGISTER_RAX = 131072;
        public const int HV_X64_REGISTER_RBX = 131075;
        public const int HV_X64_REGISTER_RIP = 131088;
        public const int HV_X64_REGISTER_RFLAGS = 131089;
        public const int HV_X64_REGISTER_CS = 393217;
        public const int HV_X64_REGISTER_RSP = 131076;
        public const int HV_X64_REGISTER_RCX = 131073;
        public const int HV_X64_REGISTER_RDX = 131074;
        public const int HV_X64_REGISTER_R8 = 131080;
        public const int HV_X64_REGISTER_R9 = 131081;

        public const int HV_MAP_GPA_READABLE = 1;
        public const int HV_MAP_GPA_WRITABLE = 2;
        public const int HV_MAP_GPA_EXECUTABLE = 12;

        public const uint HV_MESSAGE_TYPE_HVMSG_UNMAPPED_GPA = 2147483648;
        public const uint HV_MESSAGE_TYPE_HVMSG_GPA_INTERCEPT = 2147483649;
        public const uint HV_MESSAGE_TYPE_HVMSG_X64_IO_PORT_INTERCEPT = 2147549184;
        public const uint HV_MESSAGE_TYPE_HVMSG_X64_HALT = 2147549191;

        [StructLayout(LayoutKind.Sequential)]
        public struct MSHV_REGISTER
        {
            public uint Name;
            public uint Reserved1;
            public ulong Reserved2;
            public MSHV_U128 Value;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct MSHV_U128
        {
            public ulong LowPart;
            public ulong HighPart;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct MSHV_RUN_MESSAGE
        {
            public uint MessageType;
            public ulong RAX;
            public ulong RIP;
            public ushort PortNumber;
            public uint InstructionLength;
        }
        //TODO: investigate why we cannot use SafeDirectory here
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern IntPtr context_new();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        public static extern bool handle_free(IntPtr context, IntPtr handle);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern void context_free(IntPtr context);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        public static extern bool handle_is_error(IntPtr handle);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        [return: MarshalAs(UnmanagedType.U1)]
        public static extern bool is_hyperv_linux_present([MarshalAs(UnmanagedType.U1)] bool require_stable_api);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern IntPtr open_mshv(IntPtr context, bool require_stable_api);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern IntPtr create_vm(IntPtr context, IntPtr mshv_handle);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern IntPtr create_vcpu(IntPtr context, IntPtr vmfd_handle);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern IntPtr set_registers(IntPtr context, IntPtr vcpufd_handle, [In] MSHV_REGISTER[] reg_ptr, UIntPtr reg_length);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern IntPtr map_vm_memory_region(IntPtr context, IntPtr vcpufd_handle, ulong guestPfn, ulong hostAddress, ulong size);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern IntPtr unmap_vm_memory_region(IntPtr context, IntPtr vcpufd_handle, IntPtr user_memory_region_handle);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern IntPtr run_vcpu(IntPtr context, IntPtr vcpufd_handle);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern IntPtr get_run_result_from_handle(IntPtr context, IntPtr mshv_run_message_handle);

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        public static extern void free_run_result(IntPtr mshv_run_message);

#pragma warning restore CA5393

        public static bool IsHypervisorPresent()
        {
            return is_hyperv_linux_present(LinuxHyperV.REQUIRE_STABLE_API);
        }
    }
}
