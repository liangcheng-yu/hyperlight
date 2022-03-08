using System;
using System.Runtime.InteropServices;

namespace Hyperlight.Native
{
    static class LinuxKVM
    {
        [DllImport("libc", SetLastError = true)]
        public static extern int ioctl(int fd, ulong request, [In][Out] ref KVM_SREGS sregs);
        [DllImport("libc", SetLastError = true)]
        public static extern int ioctl(int fd, ulong request, [In][Out] ref KVM_REGS regs);
        [DllImport("libc", SetLastError = true)]
        public static extern int ioctl(int fd, ulong request, [In][Out] ref KVM_USERSPACE_MEMORY_REGION region);

        [DllImport("libc", SetLastError = true)]
        public static extern int ioctl(int fd, ulong request, ulong arg1);


        [DllImport("libc", SetLastError = true)]
        public static extern int open(string path, int flags);

        [DllImport("libc", SetLastError = true)]
        public static extern int close(int fd);

        public const int O_RDWR = 2;
        public const int O_CLOEXEC = 0x80000;

        const int IOC_READ = 2;
        const int IOC_WRITE = 1;

        const int IOC_NRBITS = 8;
        const int IOC_TYPEBITS = 8;
        const int IOC_SIZEBITS = 14;

        const int IOC_NRSHIFT = 0;
        const int IOC_TYPESHIFT = IOC_NRSHIFT + IOC_NRBITS;
        const int IOC_SIZESHIFT = IOC_TYPESHIFT + IOC_TYPEBITS;
        const int IOC_DIRSHIFT = IOC_SIZESHIFT + IOC_SIZEBITS;

        const uint KVMIO = 0xAE;

        public const ulong KVM_GET_API_VERSION = (KVMIO << IOC_TYPESHIFT) + (0x00 << IOC_NRSHIFT);
        public const ulong KVM_CREATE_VM = (KVMIO << IOC_TYPESHIFT) + (0x01 << IOC_NRSHIFT);
        public const ulong KVM_CHECK_EXTENSION = (KVMIO << IOC_TYPESHIFT) + (0x03 << IOC_NRSHIFT);
        public const ulong KVM_GET_VCPU_MMAP_SIZE = (KVMIO << IOC_TYPESHIFT) + (0x04 << IOC_NRSHIFT);
        public const ulong KVM_CREATE_VCPU = (KVMIO << IOC_TYPESHIFT) + (0x41 << IOC_NRSHIFT);
        public const ulong KVM_RUN_REQ = (KVMIO << IOC_TYPESHIFT) + (0x80 << IOC_NRSHIFT);

        public static ulong KVM_GET_SREGS = (ulong)((IOC_READ << IOC_DIRSHIFT) + (KVMIO << IOC_TYPESHIFT) + (0x83 << IOC_NRSHIFT) + (Marshal.SizeOf<KVM_SREGS>() << IOC_SIZESHIFT));
        public static ulong KVM_SET_SREGS = (ulong)((IOC_WRITE << IOC_DIRSHIFT) + (KVMIO << IOC_TYPESHIFT) + (0x84 << IOC_NRSHIFT) + (Marshal.SizeOf<KVM_SREGS>() << IOC_SIZESHIFT));

        public static ulong KVM_SET_USER_MEMORY_REGION = (ulong)((IOC_WRITE << IOC_DIRSHIFT) + (KVMIO << IOC_TYPESHIFT) + (0x46 << IOC_NRSHIFT) + (Marshal.SizeOf<KVM_USERSPACE_MEMORY_REGION>() << IOC_SIZESHIFT));
        public static ulong KVM_SET_REGS = (ulong)((IOC_WRITE << IOC_DIRSHIFT) + (KVMIO << IOC_TYPESHIFT) + (0x82 << IOC_NRSHIFT) + (Marshal.SizeOf<KVM_REGS>() << IOC_SIZESHIFT));
        public static ulong KVM_GET_REGS = (ulong)((IOC_READ << IOC_DIRSHIFT) + (KVMIO << IOC_TYPESHIFT) + (0x81 << IOC_NRSHIFT) + (Marshal.SizeOf<KVM_REGS>() << IOC_SIZESHIFT));

        const int KVM_CAP_USER_MEMORY = 3;

        public const int KVM_EXIT_IO = 2;
        public const int KVM_EXIT_HLT = 5;

        public const int KVM_EXIT_IO_IN = 0;
        public const int KVM_EXIT_IO_OUT = 1;

        [StructLayout(LayoutKind.Sequential)]
        public struct KVM_SEGMENT
        {
            public ulong Base;
            public uint limit;
            public ushort selector;
            public byte type;
            public byte present, dpl, db, s, l, g, avl;
            public byte unusable;
            public byte padding;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct KVM_USERSPACE_MEMORY_REGION
        {
            public uint slot;
            public uint flags;
            public ulong guest_phys_addr;
            public ulong memory_size;
            public ulong userspace_addr;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct KVM_DTABLE
        {
            public ulong Base;
            public ushort limit;
            public ushort padding1, padding2, padding3;
        }
        [StructLayout(LayoutKind.Sequential)]
        public struct KVM_SREGS
        {
            public KVM_SEGMENT cs, ds, es, fs, gs, ss;
            public KVM_SEGMENT tr, ldt;
            public KVM_DTABLE gdt, idt;
            public ulong cr0, cr2, cr3, cr4, cr8;
            public ulong efer;
            public ulong apic_base;
            public ulong interrupt_bitmap1, interrupt_bitmap2, interrupt_bitmap3, interrupt_bitmap4; // 32 320
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct KVM_REGS
        {
            public ulong rax, rbx, rcx, rdx;
            public ulong rsi, rdi, rsp, rbp;
            public ulong r8, r9, r10, r11;
            public ulong r12, r13, r14, r15;
            public ulong rip, rflags;
        };

        [StructLayout(LayoutKind.Sequential)]
        public struct KVM_RUN
        {
            public byte request_interrupt_window;
            public byte immediate_exit;
            public byte padding1_1, padding1_2, padding1_3, padding1_4, padding1_5, padding1_6;

            /* out */
            public uint exit_reason;
            public byte ready_for_interrupt_injection;
            public byte if_flag;
            public ushort flags;

            /* in (pre_kvm_run), out (post_kvm_run) */
            public ulong cr8;
            public ulong apic_base;

            // io
            public byte direction;
            public byte size; /* bytes */
            public ushort port;
            public uint count;
            public ulong data_offset; /* relative to kvm_run start */
        }


        public static bool IsHypervisorPresent()
        {
            var kvm = -1;
            try
            {
                //TODO: Create exceptions for errors.

                if (-1 == (kvm = open("/dev/kvm", O_RDWR | O_CLOEXEC)))
                {
                    Console.Error.WriteLine("Unable to open '/dev/kvm'");
                    return false;
                }

                var kvmApiVersion = ioctl(kvm, KVM_GET_API_VERSION, 0);
                if (-1 == kvmApiVersion)
                {
                    Console.Error.WriteLine("KVM_GET_API_VERSION returned -1");
                    return false;
                }
                if (12 != kvmApiVersion)
                {
                    Console.WriteLine("KVM API Version was not 12 as expected");
                    return false;
                }

                var capUserMemory = ioctl(kvm, KVM_CHECK_EXTENSION, KVM_CAP_USER_MEMORY);
                if (-1 == capUserMemory)
                {
                    Console.Error.WriteLine("KVM_CHECK_EXTENSION/KVM_CAP_USER_MEMORY returned -1");
                    return false;
                }
                if (0 == capUserMemory)
                {
                    Console.Error.WriteLine("KVM_CAP_USER_MEMORY not available");
                    return false;
                }
            }
            finally
            {
                if (kvm != -1)
                {
                    // TODO: Handle error
                    _ = close(kvm);
                }
            }
            return true;
        }
    }
}
