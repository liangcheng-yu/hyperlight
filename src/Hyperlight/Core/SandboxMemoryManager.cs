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

        internal void WriteResponseFromHostMethodCall(Type type, object? returnValue)
        {
            // TODO: support returing different types from host method call remove all the casts. 

            var inputDataAddress = (IntPtr)this.sandboxMemoryLayout.inputDataBufferOffset;
            if (type == typeof(int))
            {
                this.SharedMem.WriteInt32(inputDataAddress, returnValue is null ? 0 : (int)returnValue);
            }
            else if (type == typeof(uint))
            {
                var result = (int)(returnValue is null ? 0 : (uint)returnValue);
                this.SharedMem.WriteInt32(inputDataAddress, result);
            }
            else if (type == typeof(long))
            {
                var result = (ulong)(returnValue is null ? 0 : (long)returnValue);
                this.SharedMem.WriteInt64(inputDataAddress, result);
            }
            else if (type == typeof(IntPtr))
            {
                var result = (ulong)(returnValue is null ? 0 : ((IntPtr)returnValue).ToInt64());
                this.SharedMem.WriteInt64(inputDataAddress, result);
            }
            else if (type == typeof(void))
            {
                return;
            }
            else
            {
                HyperlightException.LogAndThrowException<ArgumentException>($"Unsupported Host Method Return Type {type.FullName}", GetType().Name);
            }
        }

        internal GuestLogData ReadGuestLogData()
        {
            var offset = GetAddressOffset();
            var outputDataAddress = this.sandboxMemoryLayout.GetOutputDataAddress(SourceAddress);
            return GuestLogData.Create(outputDataAddress, offset);
        }
    }
}
