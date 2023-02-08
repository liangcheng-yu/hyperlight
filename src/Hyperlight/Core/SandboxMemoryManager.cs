using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Native;
using Hyperlight.Wrapper;

namespace Hyperlight.Core
{
    internal class SandboxMemoryManager : Wrapper.SandboxMemoryManager
    {
        private bool disposedValue;

        internal SandboxMemoryManager(
            Context ctx,
            SandboxMemoryConfiguration sandboxMemoryConfiguration,
            bool runFromProcessMemory = false
        ) : base(
            ctx,
            sandboxMemoryConfiguration,
            runFromProcessMemory
        )
        {
        }

        internal void LoadGuestBinaryUsingLoadLibrary(string guestBinaryPath, PEInfo peInfo)
        {
            HyperlightException.ThrowIfNull(guestBinaryPath, nameof(guestBinaryPath), GetType().Name);
            HyperlightException.ThrowIfNull(peInfo, nameof(peInfo), GetType().Name);

            var headers = peInfo.GetHeaders();
            this.sandboxMemoryLayout = new SandboxMemoryLayout(
                this.ContextWrapper,
                this.sandboxMemoryConfiguration,
                0,
                (ulong)headers.StackReserve,
                (ulong)headers.HeapReserve
            );
            this.size = sandboxMemoryLayout.GetMemorySize();
            this.sharedMemoryWrapper = new SharedMemory(this.ContextWrapper, this.Size);

            loadAddress = OS.LoadLibrary(guestBinaryPath);

            // Mark first byte as 'J' so we know we are running in hyperlight VM and not as real windows exe
            Marshal.WriteByte(loadAddress, (byte)'J');

            EntryPoint = (ulong)loadAddress + headers.EntryPointOffset;

            this.sourceAddress = this.sharedMemoryWrapper.Address;

            if (IntPtr.Zero == SourceAddress)
            {
                HyperlightException.LogAndThrowException("Memory allocation failed", GetType().Name);
            }

            // Write a pointer to code so that guest exe can check that it is running in Hyperlight
            this.sharedMemoryWrapper.WriteInt64(
                (IntPtr)sandboxMemoryLayout.codePointerAddressOffset,
                (ulong)loadAddress
            );
        }
        internal void LoadGuestBinaryIntoMemory(PEInfo peInfo)
        {
            HyperlightException.ThrowIfNull(peInfo, nameof(peInfo), GetType().Name);

            var headers = peInfo.GetHeaders();
            sandboxMemoryLayout = new SandboxMemoryLayout(
                this.ContextWrapper,
                this.sandboxMemoryConfiguration,
                (ulong)peInfo.PayloadLength,
                (ulong)headers.StackReserve,
                (ulong)headers.HeapReserve
            );
            this.size = sandboxMemoryLayout.GetMemorySize();
            this.sharedMemoryWrapper = new SharedMemory(this.ContextWrapper, this.Size);
            this.sourceAddress = this.sharedMemoryWrapper.Address;

            // If we are running in the host process, then the entry point will be relative to the host memory.
            // If we are running in a hypervisor, then it's relative the guest memory.
            var addressToLoadAt = runFromProcessMemory ? (ulong)SandboxMemoryLayout.GetHostCodeAddress(SourceAddress) : SandboxMemoryLayout.GuestCodeAddress;
            EntryPoint = addressToLoadAt + headers.EntryPointOffset;

            // Copy the PE file, applying relocations if required
            var relocatedPayload = peInfo.Relocate(addressToLoadAt);
            this.sharedMemoryWrapper.CopyFromByteArray(
                relocatedPayload,
                (IntPtr)SandboxMemoryLayout.CodeOffSet
            );

            // Write a pointer to code so that guest exe can check that it is running in Hyperlight
            this.sharedMemoryWrapper.WriteInt64(
                (IntPtr)sandboxMemoryLayout.codePointerAddressOffset,
                addressToLoadAt
            );

        }

        internal HyperlightPEB SetUpHyperLightPEB()
        {
            sandboxMemoryLayout!.WriteMemoryLayout(this.sharedMemoryWrapper!, GetGuestAddressFromPointer(SourceAddress), Size);
            var offset = GetAddressOffset();
            return new HyperlightPEB(sandboxMemoryLayout.GetFunctionDefinitionAddress(SourceAddress), (int)this.sandboxMemoryConfiguration.HostFunctionDefinitionSize, offset);
        }

        internal (GuestErrorCode ErrorCode, string? Message) GetGuestError()
        {
            var guestErrorOffset = sandboxMemoryLayout!.guestErrorOffset;
            var error = this.sharedMemoryWrapper!.ReadInt64(
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
            var outputDataAddress = sandboxMemoryLayout!.GetOutputDataAddress(SourceAddress);
            return new SimpleDataTable(outputDataAddress + headerSize, (int)this.sandboxMemoryConfiguration.OutputDataSize - headerSize, GetAddressOffset());
        }

        internal string GetHostCallMethodName()
        {
            var outputDataAddress = sandboxMemoryLayout!.outputDataBufferOffset;
            var strPtr = this.sharedMemoryWrapper!.ReadInt64((UIntPtr)outputDataAddress);
            var methodName = Marshal.PtrToStringAnsi(GetHostAddressFromPointer(strPtr));
            HyperlightException.ThrowIfNull(methodName, GetType().Name);
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
                    args[i] = this.sharedMemoryWrapper!.ReadInt32(outputDataAddress + 8 * (i + 1));
                }
                else if (parameters[i].ParameterType == typeof(string))
                {
                    strPtr = this.sharedMemoryWrapper!.ReadInt64(outputDataAddress + 8 * (i + 1));
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

            var inputDataAddress = (IntPtr)sandboxMemoryLayout!.inputDataBufferOffset;
            if (type == typeof(int))
            {
                this.sharedMemoryWrapper!.WriteInt32(inputDataAddress, returnValue is null ? 0 : (int)returnValue);
            }
            else if (type == typeof(uint))
            {
                int result = (int)(returnValue is null ? 0 : (uint)returnValue);
                this.sharedMemoryWrapper!.WriteInt32(inputDataAddress, result);
            }
            else if (type == typeof(long))
            {
                ulong result = (ulong)(returnValue is null ? 0 : (long)returnValue);
                this.sharedMemoryWrapper!.WriteInt64(inputDataAddress, result);
            }
            else if (type == typeof(IntPtr))
            {
                ulong result = (ulong)(returnValue is null ? 0 : ((IntPtr)returnValue).ToInt64());
                this.sharedMemoryWrapper!.WriteInt64(inputDataAddress, result);
            }
            else
            {
                HyperlightException.LogAndThrowException<ArgumentException>($"Unsupported Host Method Return Type {nameof(type)}", GetType().Name);
            }
        }

        internal GuestLogData ReadGuestLogData()
        {
            var offset = GetAddressOffset();
            var outputDataAddress = sandboxMemoryLayout!.GetOutputDataAddress(SourceAddress);
            return GuestLogData.Create(outputDataAddress, offset);
        }

        protected override void DisposeHook(bool disposing)
        {
            // This function is called by the parent's Dispose method.
            if (!disposedValue)
            {
                if (disposing)
                {
                    this.sandboxMemoryLayout?.Dispose();
                    this.sharedMemoryWrapper!.Dispose();
                }

                if (IntPtr.Zero != loadAddress)
                {
                    OS.FreeLibrary(loadAddress);
                }

                disposedValue = true;
            }
        }
    }
}
