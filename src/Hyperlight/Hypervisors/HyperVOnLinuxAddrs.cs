using System;
using System.Diagnostics.CodeAnalysis;
using System.Runtime.InteropServices;
using Hyperlight.Wrapper;

namespace Hyperlight.Hypervisors
{
    [StructLayout(LayoutKind.Sequential)]
    public struct HyperVOnLinuxAddrs
    {
        // IMPORTANT: do not change the order of these fields,
        // unless you change them to match in hyperv_linux_mem.rs
        public ulong entrypoint;
        public ulong guestPFN;
        public ulong hostAddr;
        public ulong memSize;
        public HyperVOnLinuxAddrs(
            ulong entrypoint,
            ulong hostAddr,
            ulong memSize
        )
        {
            this.entrypoint = entrypoint;
            this.hostAddr = hostAddr;
            this.guestPFN = (ulong)SandboxMemoryLayout.BaseAddress >> 12;
            this.memSize = memSize;
        }

        public override bool Equals(Object? obj)
        {
            //Check for null and compare run-time types.
            if ((obj == null) || !this.GetType().Equals(obj.GetType()))
            {
                return false;
            }
            HyperVOnLinuxAddrs other = (HyperVOnLinuxAddrs)obj;
            return (
                other.entrypoint == this.entrypoint &&
                other.hostAddr == this.hostAddr &&
                other.guestPFN == this.guestPFN &&
                other.memSize == this.memSize
            );
        }
        public override int GetHashCode()
        {
            return (int)(
                this.entrypoint +
                this.hostAddr +
                this.guestPFN +
                this.memSize
            );
        }

    }
}
