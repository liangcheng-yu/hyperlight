using System;

namespace Hyperlight.Hypervisors
{
    abstract class Hypervisor : IDisposable
    {
        protected readonly ulong EntryPoint;
        protected ulong rsp;
        protected Action<ushort, byte> handleoutb;
        protected Action handleMemoryAccess;
        protected IntPtr sourceAddress;

        internal Hypervisor(IntPtr sourceAddress, ulong entryPoint, ulong rsp, Action<ushort, byte> outb, Action handleMemoryAccess)
        {
            this.handleMemoryAccess = handleMemoryAccess;
            this.handleoutb = outb;
            this.EntryPoint = entryPoint;
            this.rsp = rsp;
            this.sourceAddress = sourceAddress;
        }

        internal abstract void DispatchCallFromHost(ulong pDispatchFunction);
        internal abstract void Initialise(IntPtr pebAddress, ulong seed, uint pageSize);
        internal abstract void ResetRSP(ulong rsp);
        internal void HandleOutb(ushort port, byte value)
        {
            handleoutb(port, value);
        }
        internal void HandleMemoryAccess()
        {
            handleMemoryAccess();
        }
        public abstract void Dispose();
    }
}
