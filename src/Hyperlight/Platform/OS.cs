using System;
using System.Runtime.InteropServices;

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
        public static extern IntPtr VirtualAlloc(IntPtr lpAddress, IntPtr dwSize, AllocationType flAllocationType, MemoryProtection flProtect);

        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        public static extern int VirtualFree(IntPtr lpAddress, IntPtr dwSize, uint dwFreeType);

        [DllImport("kernel32", SetLastError = true, CharSet = CharSet.Ansi)]
        public static extern IntPtr LoadLibrary([MarshalAs(UnmanagedType.LPStr)] string lpFileName);

        [DllImport("kernel32", SetLastError = true, CharSet = CharSet.Ansi)]
        public static extern IntPtr FreeLibrary(IntPtr hModule);

        [DllImport("kernel32.dll")]
        public static extern bool VirtualProtect(IntPtr lpAddress, UIntPtr dwSize, OS.MemoryProtection flNewProtect, out OS.MemoryProtection lpflOldProtect);


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
        public static extern IntPtr mmap(IntPtr addr, ulong length, int prot, int flags, int fd, ulong offset);

        [DllImport("libc", SetLastError = true)]
        public static extern IntPtr munmap(IntPtr addr, ulong length);

        public static IntPtr Allocate(IntPtr addr, ulong size)
        {
            IntPtr memPtr;
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                memPtr = OS.mmap(addr, size, OS.PROT_READ | OS.PROT_WRITE | OS.PROT_EXEC, OS.MAP_SHARED | OS.MAP_ANONYMOUS, -1, 0);
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                memPtr = OS.VirtualAlloc(addr, (IntPtr)size - 1, OS.AllocationType.Commit | OS.AllocationType.Reserve, OS.MemoryProtection.EXECUTE_READWRITE);
            }
            else
            {
                throw new NotSupportedException();
            }

            if (IntPtr.Zero == memPtr)
            {
                var err = Marshal.GetLastWin32Error();
                throw new Exception($"Failed to allocate memory. Error code: {err}");
            }

            return memPtr;
        }

        public static void Free(IntPtr addr, ulong size)
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                OS.munmap(addr, size);
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                // TODO: Handle error
                _ = OS.VirtualFree(addr, IntPtr.Zero, (uint)AllocationType.Release);
            }
            else
            {
                throw new NotSupportedException();
            }
        }
    }
}
