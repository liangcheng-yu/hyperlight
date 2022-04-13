using System;

namespace Hyperlight.Hypervisors
{
    abstract class Hypervisor : IDisposable
    {
        protected readonly ulong EntryPoint;
        protected ulong rsp;
        protected Action<ushort, byte> handleoutb;
        protected IntPtr sourceAddress;

        internal Hypervisor(IntPtr sourceAddress, ulong entryPoint, ulong rsp, Action<ushort, byte> outb)
        {
            this.handleoutb = outb;
            this.EntryPoint = entryPoint;
            this.rsp = rsp;
            this.sourceAddress = sourceAddress;
        }

        internal abstract void DispatchCallFromHost(ulong pDispatchFunction);
        internal abstract void ExecuteUntilHalt();
        internal abstract void Initialise();
        internal void HandleOutb(ushort port, byte value)
        {
            handleoutb(port, value);
        }
        public abstract void Dispose();
    }
}
