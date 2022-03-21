using System;
using System.Runtime.InteropServices;
using Hyperlight.Native;

namespace Hyperlight.Hypervisors
{
    internal class KVM : Hypervisor, IDisposable
    {
        readonly int kvm = -1;
        readonly IntPtr pRun = IntPtr.Zero;
        readonly int vcpufd = -1;
        readonly int vmfd = -1;
        internal KVM(IntPtr sourceAddress, int pml4_addr, ulong size, ulong entryPoint, ulong rsp, Action<ushort, byte> outb) : base(sourceAddress, entryPoint, rsp, outb)
        {
            if (!LinuxKVM.IsHypervisorPresent())
            {
                throw new Exception("KVM Not Present");
            }

            if (-1 == (kvm = LinuxKVM.open("/dev/kvm", LinuxKVM.O_RDWR | LinuxKVM.O_CLOEXEC)))
            {
                throw new Exception("Unable to open '/dev/kvm'");
            }

            vmfd = LinuxKVM.ioctl(kvm, LinuxKVM.KVM_CREATE_VM, 0);
            if (-1 == vmfd)
            {
                throw new Exception("KVM_CREATE_VM returned -1");
            }

            var region = new LinuxKVM.KVM_USERSPACE_MEMORY_REGION() { slot = 0, guest_phys_addr = (ulong)Sandbox.BaseAddress, memory_size = size, userspace_addr = (ulong)sourceAddress };
            // TODO: Handle error
            _ = LinuxKVM.ioctl(vmfd, LinuxKVM.KVM_SET_USER_MEMORY_REGION, ref region);
            vcpufd = LinuxKVM.ioctl(vmfd, LinuxKVM.KVM_CREATE_VCPU, 0);
            var mmap_size = LinuxKVM.ioctl(kvm, LinuxKVM.KVM_GET_VCPU_MMAP_SIZE, 0);
            pRun = OS.mmap(IntPtr.Zero, (ulong)mmap_size, OS.PROT_READ | OS.PROT_WRITE, OS.MAP_SHARED, vcpufd, 0);
            var sregs = new LinuxKVM.KVM_SREGS();
            // TODO: Handle error
            _ = LinuxKVM.ioctl(vcpufd, LinuxKVM.KVM_GET_SREGS, ref sregs);

            sregs.cr3 = (ulong)pml4_addr;
            sregs.cr4 = X64.CR4_PAE | X64.CR4_OSFXSR | X64.CR4_OSXMMEXCPT;
            sregs.cr0 = X64.CR0_PE | X64.CR0_MP | X64.CR0_ET | X64.CR0_NE | X64.CR0_WP | X64.CR0_AM | X64.CR0_PG;
            sregs.efer = X64.EFER_LME | X64.EFER_LMA;

            var seg = new LinuxKVM.KVM_SEGMENT()
            {
                Base = 0,
                limit = 0xffffffff,
                selector = 1 << 3,
                present = 1,
                type = 11,
                dpl = 0,
                db = 0,
                s = 1,
                l = 1,
                g = 1
            };
            sregs.cs = seg;
            seg.type = 3;
            seg.selector = 2 << 3;
            sregs.ds = seg;
            sregs.es = seg;
            sregs.fs = seg;
            sregs.gs = seg;
            sregs.ss = seg;

            // TODO: Handle error
            _ = LinuxKVM.ioctl(vcpufd, LinuxKVM.KVM_SET_SREGS, ref sregs);

        }
        private bool disposedValue;

        internal override void DispatchCallFromHost(ulong pDispatchFunction)
        {
            // Move rip to the DispatchFunction pointer
            var regs = new LinuxKVM.KVM_REGS();
            // TODO: Handle error
            _ = LinuxKVM.ioctl(vcpufd, LinuxKVM.KVM_GET_REGS, ref regs);
            regs.rip = pDispatchFunction;
            // TODO: Handle error
            _ = LinuxKVM.ioctl(vcpufd, LinuxKVM.KVM_SET_REGS, ref regs);
            ExecuteUntilHalt();
        }

        internal override void ExecuteUntilHalt()
        {
            while (true)
            {
                // TODO: Handle error
                _ = LinuxKVM.ioctl(vcpufd, LinuxKVM.KVM_RUN_REQ, 0);
                var run = Marshal.PtrToStructure<LinuxKVM.KVM_RUN>(pRun);
                switch (run.exit_reason)
                {
                    case LinuxKVM.KVM_EXIT_HLT:
                        return;
                    case LinuxKVM.KVM_EXIT_IO:
                        // Save rip, call HandleOutb, then restore rip
                        var regs = new LinuxKVM.KVM_REGS();
                        // TODO: Handle error
                        _ = LinuxKVM.ioctl(vcpufd, LinuxKVM.KVM_GET_REGS, ref regs);
                        var ripOrig = regs.rip;
                        HandleOutb(run.port, Marshal.ReadByte(pRun, (int)run.data_offset));

                        // TODO: Handle error
                        // Restore rip
                        _ = LinuxKVM.ioctl(vcpufd, LinuxKVM.KVM_GET_REGS, ref regs);
                        regs.rip = ripOrig;
                        // TODO: Handle error
                        _ = LinuxKVM.ioctl(vcpufd, LinuxKVM.KVM_SET_REGS, ref regs);
                        break;
                    default:
                        throw new Exception($"Unknown exit_reason = {run.exit_reason}");
                }
            }
        }

        internal override void Run(int argument1, int argument2, int argument3)
        {
            var regs = new LinuxKVM.KVM_REGS()
            {
                rip = EntryPoint,
                rsp = rsp,
                rcx = (ulong)Sandbox.BaseAddress,
                rdx = (ulong)argument1,
                r8 = (ulong)argument2,
                r9 = (ulong)argument3,
                rflags = 0x0002,
            };
            // TODO: Handle error
            _ = LinuxKVM.ioctl(vcpufd, LinuxKVM.KVM_SET_REGS, ref regs);
            ExecuteUntilHalt();
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    // TODO: dispose managed state (managed objects)
                }
                // TODO:Should the vcpufd and the vmfd be closed?
                // see: https://www.kernel.org/doc/html/latest/virt/kvm/api.html#file-descriptors

                if (vcpufd != -1)
                {
                    // TODO: Handle error
                    _ = LinuxKVM.close(vcpufd);
                }

                if (vmfd != -1)
                {
                    // TODO: Handle error
                    _ = LinuxKVM.close(vmfd);
                }

                if (kvm != -1)
                {
                    // TODO: Handle error
                    _ = LinuxKVM.close(kvm);
                }

                disposedValue = true;
            }
        }

        ~KVM()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: false);
        }

        public override void Dispose()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }
    }
}
