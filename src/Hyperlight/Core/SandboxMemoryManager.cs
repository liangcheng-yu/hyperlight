using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Native;
using Hyperlight.Wrapper;

namespace Hyperlight.Core
{
    internal sealed class SandboxMemoryManager : Wrapper.SandboxMemoryManager
    {
        private bool disposedValue;
        private readonly LoadLibrary? loadedLib;

        private SandboxMemoryManager(
            Context ctx,
            SandboxMemoryConfiguration memCfg,
            SandboxMemoryLayout memLayout,
            SharedMemory sharedMem,
            IntPtr loadAddr,
            ulong entrypointOffset,
            bool runFromProcessMemory
        ) : base(
            ctx,
            memCfg,
            memLayout,
            sharedMem,
            loadAddr,
            entrypointOffset,
            runFromProcessMemory
        )
        { }

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
            var sandboxMemoryLayout = new SandboxMemoryLayout(
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
        internal static SandboxMemoryManager LoadGuestBinaryIntoMemory(
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
            var sandboxMemoryLayout = new SandboxMemoryLayout(
                ctx,
                memCfg,
                (ulong)peInfo.PayloadLength,
                (ulong)headers.StackReserve,
                (ulong)headers.HeapReserve
            );
            var memSize = sandboxMemoryLayout.GetMemorySize();
            var sharedMemoryWrapper = new SharedMemory(ctx, memSize);
            var sourceAddress = sharedMemoryWrapper.Address;

            // If we are running in the host process, then the entry point will be relative to the host memory.
            // If we are running in a hypervisor, then it's relative the guest memory.
            var addressToLoadAt = SandboxMemoryLayout.GuestCodeAddress;
            if (runFromProcessMemory)
            {
                addressToLoadAt = (ulong)SandboxMemoryLayout.GetHostCodeAddress(
                    sourceAddress
                );
            }
            var entrypointOffset = headers.EntryPointOffset;

            // Copy the PE file, applying relocations if required
            var relocatedPayload = peInfo.Relocate(addressToLoadAt);
            sharedMemoryWrapper.CopyFromByteArray(
                relocatedPayload,
                (IntPtr)SandboxMemoryLayout.CodeOffSet
            );

            // Write a pointer to code so that guest exe can check that it is running in Hyperlight
            sharedMemoryWrapper.WriteInt64(
                (IntPtr)sandboxMemoryLayout.codePointerAddressOffset,
                addressToLoadAt
            );

            return new SandboxMemoryManager(
                ctx,
                memCfg,
                sandboxMemoryLayout,
                sharedMemoryWrapper,
                new IntPtr((long)addressToLoadAt),
                entrypointOffset,
                runFromProcessMemory
            );
        }

        internal HyperlightPEB SetUpHyperLightPEB()
        {
            this.sandboxMemoryLayout.WriteMemoryLayout(
                this.SharedMem,
                GetGuestAddressFromPointer(SourceAddress),
                Size
            );
            var offset = GetAddressOffset();
            return new HyperlightPEB(
                this.sandboxMemoryLayout.GetFunctionDefinitionAddress(SourceAddress),
                (int)this.MemConfig.HostFunctionDefinitionSize, offset);
        }

        internal void WriteGuestFunctionCallDetails(string functionName, object[] args)
        {
            // The number of parameters to a guest function is fixed as serialisation of an array to memory
            // requires a fixed size 

            // TODO: Fix this by exlcuding the the array from the serilalised structure and then add the array at the end of the memory block see
            //  https://social.msdn.microsoft.com/Forums/vstudio/en-US/68a95a5f-07cd-424d-bf22-7cda2816b7bc/marshalstructuretoptr-where-struct-contains-a-byte-array-of-unknown-size?forum=clr
            var guestFunctionCall = new GuestFunctionCall();
            var guestArguments = new GuestArgument[Constants.MAX_NUMBER_OF_GUEST_FUNCTION_PARAMETERS];
            guestFunctionCall.guestArguments = guestArguments;
            var headerSize = Marshal.SizeOf(guestFunctionCall);
            var dataTable = GetGuestCallDataTable(headerSize);
            var outputDataAddress = this.sandboxMemoryLayout.GetOutputDataAddress(SourceAddress);
            guestFunctionCall.pFunctionName = dataTable.AddString(functionName);
            guestFunctionCall.argc = (ulong)args.Length;
            var nextArgShouldBeArrayLength = false;
            var nextArgLength = 0;
            for (var i = 0; i < Constants.MAX_NUMBER_OF_GUEST_FUNCTION_PARAMETERS; i++)
            {
                if (i >= args.Length)
                {
                    guestArguments[i].argv = 0;
                    guestArguments[i].argt = ParameterKind.none;
                }
                else
                {
                    if (nextArgShouldBeArrayLength)
                    {
                        if (args[i].GetType() == typeof(int))
                        {
                            var val = (int)args[i];
                            if (nextArgLength != val)
                            {
                                HyperlightException.LogAndThrowException<ArgumentException>($"Array length {val} does not match expected length {nextArgLength}.", GetType().Name);
                            }
                            guestArguments[i].argv = (ulong)val;
                            guestArguments[i].argt = ParameterKind.i32;
                            nextArgShouldBeArrayLength = false;
                            nextArgLength = 0;
                        }
                        else
                        {
                            HyperlightException.LogAndThrowException<ArgumentException>($"Argument {i} is not an int, the length of the array must follow the array itself", GetType().Name);
                        }
                    }
                    else
                    {
                        if (args[i].GetType() == typeof(int))
                        {
                            var val = (int)args[i];
                            guestArguments[i].argv = (ulong)val;
                            guestArguments[i].argt = ParameterKind.i32;
                        }
                        else if (args[i].GetType() == typeof(long))
                        {
                            var val = (long)args[i];
                            guestArguments[i].argv = (ulong)val;
                            guestArguments[i].argt = ParameterKind.i64;
                        }
                        else if (args[i].GetType() == typeof(string))
                        {
                            guestArguments[i].argv = dataTable.AddString((string)args[i]);
                            guestArguments[i].argt = ParameterKind.str;
                        }
                        else if (args[i].GetType() == typeof(bool))
                        {
                            var val = (bool)args[i];
                            guestArguments[i].argv = Convert.ToUInt64(val);
                            guestArguments[i].argt = ParameterKind.boolean;
                        }
                        else if (args[i].GetType() == typeof(byte[]))
                        {
                            var val = (byte[])args[i];
                            guestArguments[i].argv = dataTable.AddBytes(val);
                            guestArguments[i].argt = ParameterKind.bytearray;
                            nextArgShouldBeArrayLength = true;
                            nextArgLength = val.Length;
                        }
                        else
                        {
                            HyperlightException.LogAndThrowException<ArgumentException>("Unsupported parameter type", GetType().Name);
                        }
                    }
                }
            }
            if (nextArgShouldBeArrayLength)
            {
                HyperlightException.LogAndThrowException<ArgumentException>("Array length must be specified", GetType().Name);
            }
            Marshal.StructureToPtr(guestFunctionCall, outputDataAddress, false);
        }

        SimpleDataTable GetGuestCallDataTable(int headerSize)
        {
            var outputDataAddress = this.sandboxMemoryLayout.GetOutputDataAddress(SourceAddress);
            return new SimpleDataTable(
                outputDataAddress + headerSize,
                (int)this.MemConfig.OutputDataSize - headerSize,
                GetAddressOffset()
            );
        }

        internal object[] GetHostCallArgs(ParameterInfo[] parameters)
        {
            long strPtr;
            var args = new object[parameters.Length];
            var outputDataAddress = (UIntPtr)this.sandboxMemoryLayout.outputDataBufferOffset;
            for (var i = 0; i < parameters.Length; i++)
            {
                if (parameters[i].ParameterType == typeof(int))
                {
                    args[i] = this.SharedMem.ReadInt32(outputDataAddress + 8 * (i + 1));
                }
                else if (parameters[i].ParameterType == typeof(string))
                {
                    strPtr = this.SharedMem.ReadInt64(outputDataAddress + 8 * (i + 1));
                    var arg = Marshal.PtrToStringAnsi(GetHostAddressFromPointer(strPtr));
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
