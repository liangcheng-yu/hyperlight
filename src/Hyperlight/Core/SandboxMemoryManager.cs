using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Wrapper;

namespace Hyperlight.Core
{
    internal sealed class SandboxMemoryManager : Wrapper.SandboxMemoryManager
    {
        private bool disposedValue;
        private readonly LoadLibrary? loadedLib;

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

        private SandboxMemoryManager(
            Context ctx,
            SandboxMemoryConfiguration memCfg,
            SandboxMemoryLayout memLayout,
            SharedMemory sharedMem,
            LoadLibrary loadedLib,
            ulong entrypointOffset,
            bool runFromProcessMemory
        ) : base(
            ctx,
            memCfg,
            memLayout,
            sharedMem,
            loadedLib.LoadAddr,
            entrypointOffset,
            runFromProcessMemory
        )
        {
            this.loadedLib = loadedLib;
        }

        internal static SandboxMemoryManager LoadGuestBinaryUsingLoadLibrary(
            Context ctx,
            SandboxMemoryConfiguration memCfg,
            string guestBinaryPath,
            bool runFromProcessMemory
        )
        {
            HyperlightException.ThrowIfNull(
                ctx,
                nameof(ctx),
                MethodBase.GetCurrentMethod()!.DeclaringType!.Name
            );
            HyperlightException.ThrowIfNull(
                memCfg,
                nameof(memCfg),
                MethodBase.GetCurrentMethod()!.DeclaringType!.Name
            );
            HyperlightException.ThrowIfNull(
                guestBinaryPath,
                nameof(guestBinaryPath),
                MethodBase.GetCurrentMethod()!.DeclaringType!.Name
            );
            using var peInfo = new PEInfo(ctx, guestBinaryPath);
            var headers = peInfo.GetHeaders();
            var sandboxMemoryLayout = SandboxMemoryLayout.FromPieces(
                ctx,
                memCfg,
                0,
                headers.StackReserve,
                headers.HeapReserve
            );
            var memSize = sandboxMemoryLayout.GetMemorySize();
            var sharedMemoryWrapper = new SharedMemory(ctx, memSize);
            if (IntPtr.Zero == sharedMemoryWrapper.Address)
            {
                HyperlightException.LogAndThrowException(
                    "Memory allocation failed",
                    MethodBase.GetCurrentMethod()!.DeclaringType!.Name
                );
            }

            var loadedLib = new LoadLibrary(guestBinaryPath);

            // Mark first byte as 'J' so we know we are running in hyperlight VM and not as real windows exe
            Marshal.WriteByte(loadedLib.LoadAddr, (byte)'J');

            var entrypointOffset = headers.EntryPointOffset;
            var entrypoint = (ulong)loadedLib.LoadAddr + headers.EntryPointOffset;


            // Write a pointer to code so that guest exe can check that it is running in Hyperlight
            sharedMemoryWrapper.WriteInt64(
                (IntPtr)sandboxMemoryLayout.codePointerAddressOffset,
                (ulong)loadedLib.LoadAddr
            );

            return new SandboxMemoryManager(
                ctx,
                memCfg,
                sandboxMemoryLayout,
                sharedMemoryWrapper,
                loadedLib,
                entrypointOffset,
                runFromProcessMemory
            );
        }


        internal HyperlightPEB SetUpHyperLightPEB()
        {
            var addr = GetGuestAddressFromPointer(SourceAddress);
            if (this.RunFromProcessMemory)
            {
                addr = SourceAddress;
            }
            this.sandboxMemoryLayout.WriteMemoryLayout(
                this.SharedMem,
                addr,
                Size
            );
            var offset = GetAddressOffset();
            return new HyperlightPEB(
                this.sandboxMemoryLayout.GetFunctionDefinitionAddress(SourceAddress),
                (int)this.MemConfig.HostFunctionDefinitionSize, offset);
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

        protected override void DisposeHook(bool disposing)
        {
            // This function is called by the parent's Dispose method.
            if (!disposedValue)
            {
                if (disposing)
                {
                    this.sandboxMemoryLayout.Dispose();
                    this.SharedMem.Dispose();
                    this.loadedLib?.Dispose();
                }
                disposedValue = true;
            }
        }
    }
}
