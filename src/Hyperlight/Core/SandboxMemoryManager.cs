using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Wrapper;

namespace Hyperlight.Core
{
    internal sealed class SandboxMemoryManager : Wrapper.SandboxMemoryManager
    {
        private SandboxMemoryManager(
            Context ctx,
            Handle hdl
        ) : base(ctx, hdl)
        {
        }

        public static SandboxMemoryManager FromHandle(
            Context ctx,
            Handle hdl
        )
        {
            hdl.ThrowIfError();
            return new SandboxMemoryManager(ctx, hdl);
        }
        internal GuestLogData ReadGuestLogData()
        {
            var offset = GetAddressOffset();
            var outputDataAddress = this.sandboxMemoryLayout.GetOutputDataAddress(SourceAddress);
            return GuestLogData.Create(outputDataAddress, offset);
        }
    }
}
