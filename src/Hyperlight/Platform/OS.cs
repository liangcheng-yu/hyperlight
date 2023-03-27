using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Xml.Linq;
using Hyperlight.Core;

namespace Hyperlight.Native
{
    static class OS
    {
        //////////////////////////////////////////////////////////////////////
        // Windows memory management functions

        [Flags]
        public enum AllocationType
        {
            Commit = 0x1000,
            Reserve = 0x2000,
            Decommit = 0x4000,
            Release = 0x8000,
            Reset = 0x80000,
            Physical = 0x400000,
            TopDown = 0x100000,
            WriteWatch = 0x200000,
            LargePages = 0x20000000
        }
        [Flags]
        public enum MemoryProtection : uint
        {
            EXECUTE = 0x10,
            EXECUTE_READ = 0x20,
            EXECUTE_READWRITE = 0x40,
            EXECUTE_WRITECOPY = 0x80,
            NOACCESS = 0x01,
            READONLY = 0x02,
            READWRITE = 0x04,
            WRITECOPY = 0x08,
            GUARD_Modifierflag = 0x100,
            NOCACHE_Modifierflag = 0x200,
            WRITECOMBINE_Modifierflag = 0x400
        }

        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern IntPtr VirtualAlloc(IntPtr lpAddress, IntPtr dwSize, AllocationType flAllocationType, MemoryProtection flProtect);

        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern IntPtr VirtualAllocEx(IntPtr hProcess, IntPtr lpAddress, IntPtr dwSize, AllocationType flAllocationType, MemoryProtection flProtect);

        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern int VirtualFree(IntPtr lpAddress, IntPtr dwSize, uint dwFreeType);

        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern int VirtualFreeEx(IntPtr hProcess, IntPtr lpAddress, IntPtr dwSize, uint dwFreeType);

        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern bool ReadProcessMemory(IntPtr hProcess, IntPtr lpBaseAddress, IntPtr lpBuffer, IntPtr nSize, out IntPtr lpNumberOfBytesRead);
        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern bool WriteProcessMemory(IntPtr hProcess, IntPtr lpBaseAddress, IntPtr lpBuffer, IntPtr nSize, out IntPtr lpNumberOfBytesWritten);




        //////////////////////////////////////////////////////////////////////
        // Linux memory management functions

        public const int PROT_READ = 0x1; /* page can be read */
        public const int PROT_WRITE = 0x2; /* page can be written */
        public const int PROT_EXEC = 0x4; /* page can be executed */
        public const int PROT_SEM = 0x8; /* page may be used for atomic ops */
        public const int PROT_NONE = 0x0; /* page can not be accessed */

        public const int MAP_SHARED = 0x01; /* Share changes */
        public const int MAP_PRIVATE = 0x02; /* Changes are private */
        public const int MAP_ANONYMOUS = 0x20;/* don't use a file */

        [DllImport("libc", SetLastError = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.SafeDirectories)]
        public static extern IntPtr mmap(IntPtr addr, ulong length, int prot, int flags, int fd, ulong offset);

        [DllImport("libc", SetLastError = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.SafeDirectories)]
        public static extern IntPtr munmap(IntPtr addr, ulong length);

        // Windows Job and Process management structures and functions

        [StructLayout(LayoutKind.Sequential)]
        public sealed class SecurityAttributes
        {
            public int nLength;
            public IntPtr lpSecurityDescriptor = IntPtr.Zero;
            public bool bInheritHandle;
            public SecurityAttributes()
            {
                nLength = Marshal.SizeOf(this);
            }
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct IoCounters
        {
            public ulong ReadOperationCount;
            public ulong WriteOperationCount;
            public ulong OtherOperationCount;
            public ulong ReadTransferCount;
            public ulong WriteTransferCount;
            public ulong OtherTransferCount;
        }


        [StructLayout(LayoutKind.Sequential)]
        public struct JobBasicLimitInfo
        {
            public long PerProcessUserTimeLimit;
            public long PerJobUserTimeLimit;
            public JobLimitInfo LimitFlags;
            public UIntPtr MinimumWorkingSetSize;
            public UIntPtr MaximumWorkingSetSize;
            public uint ActiveProcessLimit;
            public UIntPtr Affinity;
            public uint PriorityClass;
            public uint SchedulingClass;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct JobExtentedLimitInfo
        {
            public JobBasicLimitInfo BasicLimitInformation;
            public IoCounters IoInfo;
            public UIntPtr ProcessMemoryLimit;
            public UIntPtr JobMemoryLimit;
            public UIntPtr PeakProcessMemoryUsed;
            public UIntPtr PeakJobMemoryUsed;
        }

        public enum JobObjectInfoType
        {
            ExtendedLimitInformation = 9,
        }

        [Flags]
        public enum JobLimitInfo : uint
        {
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000,
        }

        [Flags]
        public enum CreateProcessFlags : uint
        {
            CREATE_SUSPENDED = 0x00000004,
        }

        [StructLayout(LayoutKind.Sequential)]
        public sealed class StartupInfo
        {
            public int cb;
            public IntPtr lpReserved = IntPtr.Zero;
            public IntPtr lpDesktop = IntPtr.Zero;
            public IntPtr lpTitle = IntPtr.Zero;
            public int dwX;
            public int dwY;
            public int dwXSize;
            public int dwYSize;
            public int dwXCountChars;
            public int dwYCountChars;
            public int dwFillAttributes;
            public int dwFlags;
            public short wShowWindow;
            public short cbReserved;
            public IntPtr lpReserved2 = IntPtr.Zero;
            public IntPtr hStdInput = IntPtr.Zero;
            public IntPtr hStdOutput = IntPtr.Zero;
            public IntPtr hStdErr = IntPtr.Zero;
            public StartupInfo()
            {
                this.cb = Marshal.SizeOf(this);
            }
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct ProcessInformation
        {
            public IntPtr hProcess;
            public IntPtr hThread;
            public int dwProcessId;
            public int dwThreadId;
        }

        [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern bool CreateProcess(string? lpApplicationName, string lpCommandLine, SecurityAttributes lpProcessAttributes, SecurityAttributes lpThreadAttributes, bool bInheritHandles, CreateProcessFlags dwCreationFlags, IntPtr lpEnvironment, string? lpCurrentDirectory, [In] StartupInfo lpStartupInfo, out ProcessInformation lpProcessInformation);

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern IntPtr CreateJobObject(SecurityAttributes jobSecurityAttributes, string lpName);

        [DllImport("kernel32.dll")]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern bool SetInformationJobObject(IntPtr hJob, JobObjectInfoType infoType, IntPtr lpJobObjectInfo, uint cbJobObjectInfoLength);

        [DllImport("kernel32.dll", SetLastError = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);

        [DllImport("kernel32.dll", SetLastError = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool CloseHandle(IntPtr hObject);

        [StructLayout(LayoutKind.Sequential)]
        private struct SYSTEM_INFO
        {
            public ushort wProcessorArchitecture;
            public ushort wReserved;
            public uint dwPageSize;
            public IntPtr lpMinimumApplicationAddress;
            public IntPtr lpMaximumApplicationAddress;
            public UIntPtr dwActiveProcessorMask;
            public uint dwNumberOfProcessors;
            public uint dwProcessorType;
            public uint dwAllocationGranularity;
            public ushort wProcessorLevel;
            public ushort wProcessorRevision;
        };
        [DllImport("libc", SetLastError = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.SafeDirectories)]
        public static extern uint getpagesize();

        [DllImport("kernel32.dll", SetLastError = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        private static extern void GetNativeSystemInfo(ref SYSTEM_INFO lpSystemInfo);

        public static uint GetPageSize()
        {
            uint pageSize = 0;

            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                var sysInfo = new SYSTEM_INFO();

                GetNativeSystemInfo(ref sysInfo);

                int error;
                if ((error = Marshal.GetLastPInvokeError()) != 0)
                {
                    HyperlightException.LogAndThrowException($"GetNativeSystemInfo:Pinvoke Last Error:{error}", MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
                }

                pageSize = sysInfo.dwPageSize;
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                pageSize = Syscall.CheckReturnVal(
                    "Get PageSize",
                    () => getpagesize(),
                    (uint retVal) => retVal != 0
                );
            }
            else
            {
                HyperlightException.LogAndThrowException<NotSupportedException>($"Hyperlight is only supported on Windows and Linux", MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            }

            return pageSize;
        }

        [DllImport("kernel32.dll")]
        [DefaultDllImportSearchPaths(DllImportSearchPath.System32)]
        public static extern void GetCurrentThreadStackLimits(out IntPtr lowLimit, out IntPtr highLimit);

    }
}
