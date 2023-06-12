using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;
using Hyperlight.Wrapper;

namespace Hyperlight.Hypervisors
{
    [StructLayout(LayoutKind.Sequential)]
    public struct HypervisorAddrs : IEquatable<HypervisorAddrs>
    {
        // IMPORTANT: do not change the order of these fields,
        // unless you change them to match in hyperv_linux_mem.rs
        public ulong entrypoint;
        public ulong guestPFN;
        public ulong hostAddr;
        public ulong memSize;
        public HypervisorAddrs(
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

        public static bool operator ==(
            HypervisorAddrs a1,
            HypervisorAddrs a2
        )
        {
            return a1.Equals(a2);
        }

        public static bool operator !=(
            HypervisorAddrs a1,
            HypervisorAddrs a2
        )
        {
            return !a1.Equals(a2);
        }

        public bool Equals(HypervisorAddrs other)
        {
            return (
                other.entrypoint == this.entrypoint &&
                other.hostAddr == this.hostAddr &&
                other.guestPFN == this.guestPFN &&
                other.memSize == this.memSize
            );
        }
        public override bool Equals(object? obj)
        {
            //Check for null and compare run-time types.
            if ((obj == null) || !this.GetType().Equals(obj.GetType()))
            {
                return false;
            }
            var other = (HypervisorAddrs)obj;
            return this.Equals(other);
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
