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

        internal object[] GetHostCallArgs(ParameterInfo[] parameters)
        {
            var args = new object[parameters.Length];
            var outputDataOffset = (UIntPtr)this.sandboxMemoryLayout.outputDataBufferOffset;
            for (var i = 0; i < parameters.Length; i++)
            {
                if (parameters[i].ParameterType == typeof(int))
                {
                    args[i] = this.SharedMem.ReadInt32(outputDataOffset + 8 * (i + 1));
                }
                else if (parameters[i].ParameterType == typeof(string))
                {
                    var strPtrI64 = this.SharedMem.ReadInt64(outputDataOffset + 8 * (i + 1));
                    var strPtr = new IntPtr((long)strPtrI64);
                    if (!this.RunFromProcessMemory)
                    {
                        // if we're _not_ running with in-memory mode,
                        // the guest has written to shared memory a pointer to
                        // its address space, and not the host's.
                        //
                        // in this case, we need to translate the pointer
                        // from shared memory into a pointer to the host's
                        // address space.
                        strPtr = GetHostAddressFromPointer(strPtrI64);
                    }
                    var arg = Marshal.PtrToStringAnsi(strPtr);
                    HyperlightException.ThrowIfNull(arg, nameof(arg), GetType().Name);
                    args[i] = arg;
                }
                else
                {
                    HyperlightException.LogAndThrowException<ArgumentException>($"Unsupported parameter type: {parameters[i].ParameterType}", GetType().Name);
                }
            }
            return args;
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
            else
            {
                HyperlightException.LogAndThrowException<ArgumentException>($"Unsupported Host Method Return Type {nameof(type)}", GetType().Name);
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
