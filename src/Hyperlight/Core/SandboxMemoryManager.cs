using System;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using Hyperlight.Native;
using Hyperlight.Wrapper;
using Newtonsoft.Json;

namespace Hyperlight.Core
{
    internal class SandboxMemoryManager : IDisposable
    {
        private readonly Context ctx;
        public ulong EntryPoint { get; private set; }
        public ulong Size { get; private set; }
        public IntPtr SourceAddress { get; private set; }
        public GuestMemory? guestMemWrapper
        {
            get; private set;
        }
        private GuestMemorySnapshot? guestMemSnapshotWrapper;

        bool disposedValue;
        IntPtr loadAddress = IntPtr.Zero;
        readonly bool runFromProcessMemory;
        readonly SandboxMemoryConfiguration sandboxMemoryConfiguration;
        SandboxMemoryLayout? sandboxMemoryLayout;

        internal SandboxMemoryManager(
            Context ctx,
            SandboxMemoryConfiguration sandboxMemoryConfiguration,
            bool runFromProcessMemory = false
        )
        {
            this.ctx = ctx;
            this.sandboxMemoryConfiguration = sandboxMemoryConfiguration;
            this.runFromProcessMemory = runFromProcessMemory;
        }

        internal void LoadGuestBinaryUsingLoadLibrary(string guestBinaryPath, PEInfo peInfo)
        {
            HyperlightException.ThrowIfNull(guestBinaryPath, nameof(guestBinaryPath), Sandbox.CorrelationId.Value!, GetType().Name);
            HyperlightException.ThrowIfNull(peInfo, nameof(peInfo), Sandbox.CorrelationId.Value!, GetType().Name);

            sandboxMemoryLayout = new SandboxMemoryLayout(
                this.ctx,
                sandboxMemoryConfiguration,
                0,
                (ulong)peInfo.StackReserve,
                (ulong)peInfo.HeapReserve
            );
            Size = sandboxMemoryLayout.GetMemorySize();

            guestMemWrapper = new GuestMemory(this.ctx, this.Size);

            loadAddress = OS.LoadLibrary(guestBinaryPath);

            // Mark first byte as 'J' so we know we are running in hyperlight VM and not as real windows exe
            Marshal.WriteByte(loadAddress, (byte)'J');

            EntryPoint = (ulong)loadAddress + peInfo.EntryPointOffset;

            SourceAddress = this.guestMemWrapper.Address;

            if (IntPtr.Zero == SourceAddress)
            {
                HyperlightException.LogAndThrowException("Memory allocation failed", Sandbox.CorrelationId.Value!, GetType().Name);
            }

            // Write a pointer to code so that guest exe can check that it is running in Hyperlight

            this.guestMemWrapper.WriteInt64(
                (IntPtr)sandboxMemoryLayout.codePointerAddressOffset,
                (ulong)loadAddress
            );
        }
        internal void LoadGuestBinaryIntoMemory(PEInfo peInfo)
        {
            HyperlightException.ThrowIfNull(peInfo, nameof(peInfo), Sandbox.CorrelationId.Value!, GetType().Name);
            sandboxMemoryLayout = new SandboxMemoryLayout(
                this.ctx,
                sandboxMemoryConfiguration,
                (ulong)peInfo.Payload.Length,
                (ulong)peInfo.StackReserve,
                (ulong)peInfo.HeapReserve
            );
            Size = sandboxMemoryLayout.GetMemorySize();
            this.guestMemWrapper = new GuestMemory(this.ctx, this.Size);
            SourceAddress = this.guestMemWrapper.Address;
            var hostCodeAddress = (ulong)SandboxMemoryLayout.GetHostCodeAddress(SourceAddress);
            // If we are running in memory the entry point will be relative to the sourceAddress if we are running in a Hypervisor it will be relative to 0x230000 which is where the code is loaded in the GP
            if (runFromProcessMemory)
            {
                EntryPoint = hostCodeAddress + peInfo.EntryPointOffset;
                this.guestMemWrapper.CopyFromByteArray(
                    peInfo.Payload,
                    (IntPtr)SandboxMemoryLayout.CodeOffSet
                );

                // When loading in memory we need to fix up the relocations in the exe to reflect the address the exe was loaded at.
                peInfo.PatchExeRelocations(hostCodeAddress);

                // Write a pointer to code so that guest exe can check that it is running in Hyperlight

                this.guestMemWrapper.WriteInt64(
                    (IntPtr)sandboxMemoryLayout.codePointerAddressOffset,
                    hostCodeAddress
                );
            }
            else
            {
                EntryPoint = SandboxMemoryLayout.GuestCodeAddress + peInfo.EntryPointOffset;
                this.guestMemWrapper.CopyFromByteArray(
                    peInfo.HyperVisorPayload,
                    (IntPtr)SandboxMemoryLayout.CodeOffSet
                );

                // Write a pointer to code so that guest exe can check that it is running in Hyperlight

                this.guestMemWrapper.WriteInt64(
                    (IntPtr)sandboxMemoryLayout.codePointerAddressOffset,
                    (ulong)SandboxMemoryLayout.GuestCodeAddress
                );
            }

        }

        internal void SetStackGuard(byte[] cookie)
        {
            HyperlightException.ThrowIfNull(cookie, nameof(cookie), Sandbox.CorrelationId.Value!, GetType().Name);
            var stackOffset = sandboxMemoryLayout!.topOfStackOffset;
            this.guestMemWrapper!.CopyFromByteArray(
                cookie,
                (IntPtr)stackOffset
            );

        }

        internal bool CheckStackGuard(byte[]? cookie)
        {
            HyperlightException.ThrowIfNull(cookie, nameof(cookie), Sandbox.CorrelationId.Value!, GetType().Name);
            var guestCookie = new byte[cookie.Length];
            var stackOffset = sandboxMemoryLayout!.topOfStackOffset;
            this.guestMemWrapper!.CopyToByteArray(
                guestCookie,
                (ulong)stackOffset
            );
            return guestCookie.SequenceEqual(cookie);
        }

        internal HyperlightPEB SetUpHyperLightPEB()
        {
            sandboxMemoryLayout!.WriteMemoryLayout(this.guestMemWrapper!, GetGuestAddressFromPointer(SourceAddress), Size);
            var offset = GetAddressOffset();
            return new HyperlightPEB(sandboxMemoryLayout.GetFunctionDefinitionAddress(SourceAddress), (int)sandboxMemoryConfiguration.HostFunctionDefinitionSize, offset);
        }

        internal ulong SetUpHyperVisorPartition()
        {
            ulong rsp = Size + (ulong)SandboxMemoryLayout.BaseAddress; // Add 0x200000 because that's the start of mapped memorS

            // For MSVC, move rsp down by 0x28.  This gives the called 'main' function the appearance that rsp was
            // was 16 byte aligned before the 'call' that calls main (note we don't really have a return value on the
            // stack but some assembly instructions are expecting rsp have started 0x8 bytes off of 16 byte alignment
            // when 'main' is invoked.  We do 0x28 instead of 0x8 because MSVC can expect that there are 0x20 bytes
            // of space to write to by the called function.  I am not sure if this happens with the 'main' method, but
            // we do this just in case.
            // NOTE: We do this also for GCC freestanding binaries because we specify __attribute__((ms_abi)) on the start method
            rsp -= 0x28;

            // Create pagetable

            this.guestMemWrapper!.WriteInt64((IntPtr)SandboxMemoryLayout.PML4Offset, X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | SandboxMemoryLayout.PDPTGuestAddress);
            this.guestMemWrapper!.WriteInt64((IntPtr)SandboxMemoryLayout.PDPTOffset, X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | SandboxMemoryLayout.PDGuestAddress);

            for (var i = 0/*We do not map first 2 megs*/; i < 512; i++)
            {
                // map each VA to physical memory 2 megs lower
                var valToWrite = (
                    (i << 21) +
                    (long)(
                        X64.PDE64_PRESENT |
                        X64.PDE64_RW |
                        X64.PDE64_USER |
                        X64.PDE64_PS
                    )
                );
                this.guestMemWrapper.WriteInt64(IntPtr.Add((IntPtr)SandboxMemoryLayout.PDOffset, i * 8), (ulong)valToWrite);
            }

            return rsp;
        }

        internal void SnapshotState()
        {
            // we may not have taken a snapshot yet, but we must
            // have a guest memory by this point so we can take
            // one.
            HyperlightException.ThrowIfNull(
                this.guestMemWrapper,
                nameof(this.guestMemWrapper),
                GetType().Name
            );

            if (null == this.guestMemSnapshotWrapper)
            {
                // if we haven't snapshotted already, create a new
                // GuestMemorySnapshot from the existing guest memory.
                this.guestMemSnapshotWrapper = new GuestMemorySnapshot(
                    this.ctx,
                    this.guestMemWrapper
                );
            }
            else
            {
                // otherwise, if we have already snapshotted, replace
                // the existing snapshot we have already.
                this.guestMemSnapshotWrapper.ReplaceSnapshot();
            }
        }

        internal void RestoreState()
        {
            // we should have already created a snapshot by this
            // point, so throw if we haven't
            HyperlightException.ThrowIfNull(
                this.guestMemSnapshotWrapper,
                nameof(this.guestMemSnapshotWrapper),
                GetType().Name
            );
            this.guestMemSnapshotWrapper.RestoreFromSnapshot();
        }

        internal int GetReturnValue()
        {
            return this.guestMemWrapper!.ReadInt32(
                (UIntPtr)this.sandboxMemoryLayout!.outputDataOffset
            );
        }

        internal void SetOutBAddress(long pOutB)
        {
            var outBPointerOffset = sandboxMemoryLayout!.outbPointerOffset;
            this.guestMemWrapper!.WriteInt64((IntPtr)outBPointerOffset, (ulong)pOutB);
        }

        internal (GuestErrorCode ErrorCode, string? Message) GetGuestError()
        {
            var guestErrorOffset = sandboxMemoryLayout!.guestErrorOffset;
            var error = this.guestMemWrapper!.ReadInt64(
                (UIntPtr)guestErrorOffset
            );
            var guestErrorCode = error switch
            {
                var e when Enum.IsDefined(typeof(GuestErrorCode), e) => (GuestErrorCode)error,
                _ => GuestErrorCode.UNKNOWN_ERROR,
            };

            if (guestErrorCode == GuestErrorCode.NO_ERROR)
            {
                return (GuestErrorCode.NO_ERROR, null);
            }

            var guestErrorMessagePointerAddress = sandboxMemoryLayout.GetGuestErrorMessagePointerAddress(SourceAddress);
            var guestErrorMessageAddress = GetHostAddressFromPointer(Marshal.ReadInt64(guestErrorMessagePointerAddress));
            var errorMessage = Marshal.PtrToStringAnsi(guestErrorMessageAddress);

            if (guestErrorCode == GuestErrorCode.UNKNOWN_ERROR)
            {
                errorMessage += $":Error Code:{error}";
            }

            return (guestErrorCode, errorMessage);
        }

        internal ulong GetPointerToDispatchFunction()
        {
            return (ulong)this.guestMemWrapper!.ReadInt64((UIntPtr)sandboxMemoryLayout!.dispatchFunctionPointerOffSet);
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
            var outputDataAddress = sandboxMemoryLayout!.GetOutputDataAddress(SourceAddress);
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
                                HyperlightException.LogAndThrowException<ArgumentException>($"Array length {val} does not match expected length {nextArgLength}.", Sandbox.CorrelationId.Value!, GetType().Name);
                            }
                            guestArguments[i].argv = (ulong)val;
                            guestArguments[i].argt = ParameterKind.i32;
                            nextArgShouldBeArrayLength = false;
                            nextArgLength = 0;
                        }
                        else
                        {
                            HyperlightException.LogAndThrowException<ArgumentException>($"Argument {i} is not an int, the length of the array must follow the array itself", Sandbox.CorrelationId.Value!, GetType().Name);
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
                            HyperlightException.LogAndThrowException<ArgumentException>("Unsupported parameter type", Sandbox.CorrelationId.Value!, GetType().Name);
                        }
                    }
                }
            }
            if (nextArgShouldBeArrayLength)
            {
                HyperlightException.LogAndThrowException<ArgumentException>("Array length must be specified", Sandbox.CorrelationId.Value!, GetType().Name);
            }
            Marshal.StructureToPtr(guestFunctionCall, outputDataAddress, false);
        }

        SimpleDataTable GetGuestCallDataTable(int headerSize)
        {
            var outputDataAddress = sandboxMemoryLayout!.GetOutputDataAddress(SourceAddress);
            return new SimpleDataTable(outputDataAddress + headerSize, (int)sandboxMemoryConfiguration.OutputDataSize - headerSize, GetAddressOffset());
        }

        internal string GetHostCallMethodName()
        {
            var outputDataAddress = sandboxMemoryLayout!.outputDataBufferOffset;
            var strPtr = this.guestMemWrapper!.ReadInt64((UIntPtr)outputDataAddress);
            var methodName = Marshal.PtrToStringAnsi(GetHostAddressFromPointer(strPtr));
            HyperlightException.ThrowIfNull(methodName, Sandbox.CorrelationId.Value!, GetType().Name);
            return methodName;
        }

        internal object[] GetHostCallArgs(ParameterInfo[] parameters)
        {
            long strPtr;
            var args = new object[parameters.Length];
            var outputDataAddress = (UIntPtr)sandboxMemoryLayout!.outputDataBufferOffset;
            for (var i = 0; i < parameters.Length; i++)
            {
                if (parameters[i].ParameterType == typeof(int))
                {
                    args[i] = this.guestMemWrapper!.ReadInt32(outputDataAddress + 8 * (i + 1));
                }
                else if (parameters[i].ParameterType == typeof(string))
                {
                    strPtr = this.guestMemWrapper!.ReadInt64(outputDataAddress + 8 * (i + 1));
                    var arg = Marshal.PtrToStringAnsi(GetHostAddressFromPointer(strPtr));
                    HyperlightException.ThrowIfNull(arg, nameof(arg), Sandbox.CorrelationId.Value!, GetType().Name);
                    args[i] = arg;
                }
                else
                {
                    HyperlightException.LogAndThrowException<ArgumentException>($"Unsupported parameter type: {parameters[i].ParameterType}", Sandbox.CorrelationId.Value!, GetType().Name);
                }
            }
            return args;
        }

        internal void WriteResponseFromHostMethodCall(Type type, object? returnValue)
        {
            // TODO: support returing different types from host method call remove all the casts. 

            var inputDataAddress = (IntPtr)sandboxMemoryLayout!.inputDataBufferOffset;
            if (type == typeof(int))
            {
                this.guestMemWrapper!.WriteInt32(inputDataAddress, returnValue is null ? 0 : (int)returnValue);
            }
            else if (type == typeof(uint))
            {
                int result = (int)(returnValue is null ? 0 : (uint)returnValue);
                this.guestMemWrapper!.WriteInt32(inputDataAddress, result);
            }
            else if (type == typeof(long))
            {
                ulong result = (ulong)(returnValue is null ? 0 : (long)returnValue);
                this.guestMemWrapper!.WriteInt64(inputDataAddress, result);
            }
            else if (type == typeof(IntPtr))
            {
                ulong result = (ulong)(returnValue is null ? 0 : ((IntPtr)returnValue).ToInt64());
                this.guestMemWrapper!.WriteInt64(inputDataAddress, result);
            }
            else
            {
                HyperlightException.LogAndThrowException<ArgumentException>($"Unsupported Host Method Return Type {nameof(type)}", Sandbox.CorrelationId.Value!, GetType().Name);
            }
        }

        internal HyperlightException? GetHostException()
        {
            var hostExceptionOffset = sandboxMemoryLayout!.hostExceptionOffset;
            HyperlightException? hyperlightException = null;
            var dataLength = this.guestMemWrapper!.ReadInt32((UIntPtr)hostExceptionOffset);
            if (dataLength > 0)
            {
                var data = new byte[dataLength];
                this.guestMemWrapper.CopyToByteArray(data, (ulong)hostExceptionOffset + sizeof(int));
                var exceptionAsJson = Encoding.UTF8.GetString(data);
                // TODO: Switch to System.Text.Json - requires custom serialisation as default throws an exception when serialising if an inner exception is present
                // as it contains a Type: System.NotSupportedException: Serialization and deserialization of 'System.Type' instances are not supported and should be avoided since they can lead to security issues.
                // https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-converters-how-to?pivots=dotnet-6-0
#pragma warning disable CA2326 // Do not use TypeNameHandling values other than None - this will be fixed by the above TODO
#pragma warning disable CA2327 // Do not use SerializationBinder classes - this will be fixed by the above TODO 
                hyperlightException = JsonConvert.DeserializeObject<HyperlightException>(exceptionAsJson, new JsonSerializerSettings
                {
                    TypeNameHandling = TypeNameHandling.Auto
                });
#pragma warning restore CA2326 // Do not use TypeNameHandling values other than None
#pragma warning restore CA2327 // Do not use SerializationBinder classes
            }
            return hyperlightException;
        }

        internal IntPtr GetHostAddressFromPointer(long address)
        {
            return (IntPtr)(address + GetAddressOffset());
        }

        internal IntPtr GetGuestAddressFromPointer(IntPtr address)
        {
            return (IntPtr)((long)address - GetAddressOffset());
        }

        internal long GetAddressOffset()
        {
            return runFromProcessMemory ? 0 : (long)((ulong)SourceAddress - SandboxMemoryLayout.BaseAddress);
        }

        internal void WriteOutbException(Exception ex, ushort port)
        {
            var guestErrorAddressOffset = sandboxMemoryLayout!.guestErrorAddressOffset;
            this.guestMemWrapper!.WriteInt64((IntPtr)guestErrorAddressOffset, (long)GuestErrorCode.OUTB_ERROR);

            var guestErrorMessageBufferOffset = sandboxMemoryLayout!.guestErrorMessageBufferOffset;
            var data = Encoding.UTF8.GetBytes($"Port:{port}, Message:{ex.Message}\0");
            if (data.Length <= (int)sandboxMemoryConfiguration.GuestErrorMessageSize)
            {
                this.guestMemWrapper!.CopyFromByteArray(data, (IntPtr)guestErrorMessageBufferOffset);
            }

            var hyperLightException = ex.GetType() == typeof(HyperlightException) ? ex as HyperlightException : new HyperlightException($"OutB Error {ex.Message}", ex);
            var hostExceptionOffset = sandboxMemoryLayout!.hostExceptionOffset;

            // TODO: Switch to System.Text.Json - requires custom serialisation as default throws an exception when serialising if an inner exception is present
            // as it contains a Type: System.NotSupportedException: Serialization and deserialization of 'System.Type' instances are not supported and should be avoided since they can lead to security issues.
            // https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-converters-how-to?pivots=dotnet-6-0
#pragma warning disable CA2326 // Do not use TypeNameHandling values other than None - this will be fixed by the above TODO
            var exceptionAsJson = JsonConvert.SerializeObject(hyperLightException, new JsonSerializerSettings
            {
                TypeNameHandling = TypeNameHandling.Auto
            });
#pragma warning restore CA2326 // Do not use TypeNameHandling values other than None
            data = Encoding.UTF8.GetBytes(exceptionAsJson);
            var dataLength = data.Length;

            if (dataLength <= (int)sandboxMemoryConfiguration.HostExceptionSize - sizeof(int))
            {
                this.guestMemWrapper!.WriteInt32((IntPtr)hostExceptionOffset, dataLength);
                this.guestMemWrapper!.CopyFromByteArray(data, (IntPtr)hostExceptionOffset + sizeof(int));
            }

            HyperlightLogger.LogError($"Exception occurred in outb", Sandbox.CorrelationId.Value!, GetType().Name, ex);

        }
        internal ulong GetPebAddress()
        {
            if (runFromProcessMemory)
            {
                return (ulong)sandboxMemoryLayout!.GetInProcessPEBAddress(SourceAddress);
            }

            return (ulong)sandboxMemoryLayout!.PEBAddress;
        }

        internal string? ReadStringOutput()
        {
            var outputDataAddress = sandboxMemoryLayout!.GetOutputDataAddress(SourceAddress);
            return Marshal.PtrToStringAnsi(outputDataAddress);
        }

        internal GuestLogData ReadGuestLogData()
        {
            var offset = GetAddressOffset();
            var outputDataAddress = sandboxMemoryLayout!.GetOutputDataAddress(SourceAddress);
            return GuestLogData.Create(outputDataAddress, offset);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    this.sandboxMemoryLayout?.Dispose();
                    this.guestMemWrapper!.Dispose();
                    this.guestMemSnapshotWrapper?.Dispose();
                }

                if (IntPtr.Zero != loadAddress)
                {
                    OS.FreeLibrary(loadAddress);
                }

                disposedValue = true;
            }
        }

        // TODO: override finalizer only if 'Dispose(bool disposing)' has code to free unmanaged resources
        ~SandboxMemoryManager()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: false);
        }

        public void Dispose()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }
    }
}
