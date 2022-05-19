using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Native;
using System.Text;
using Newtonsoft.Json;
using System.Net.Http;

namespace Hyperlight.Core
{
    internal class SandboxMemoryManager : IDisposable
    {
        public ulong EntryPoint { get; private set; }
        public ulong Size { get; private set; }
        public IntPtr SourceAddress { get; private set; }

        bool disposedValue;
        IntPtr loadAddress = IntPtr.Zero;
        byte[] memorySnapShot;
        readonly bool runFromProcessMemory;
        readonly SandboxMemoryConfiguration sandboxMemoryConfiguration;
        SandboxMemoryLayout sandboxMemoryLayout;

        internal SandboxMemoryManager(bool runFromProcessMemory = false) : this(new SandboxMemoryConfiguration(), runFromProcessMemory)
        {
        }

        internal SandboxMemoryManager(SandboxMemoryConfiguration sandboxMemoryConfiguration, bool runFromProcessMemory = false)
        {
            this.sandboxMemoryConfiguration = sandboxMemoryConfiguration;
            this.runFromProcessMemory = runFromProcessMemory;
        }

        internal void LoadGuestBinaryUsingLoadLibrary(string guestBinaryPath, PEInfo peInfo)
        {
            sandboxMemoryLayout = new SandboxMemoryLayout(sandboxMemoryConfiguration, 0, peInfo.StackReserve, peInfo.HeapReserve);
            Size = sandboxMemoryLayout.GetMemorySize();
            ArgumentNullException.ThrowIfNull(guestBinaryPath, nameof(guestBinaryPath));
            ArgumentNullException.ThrowIfNull(peInfo, nameof(peInfo));

            loadAddress = OS.LoadLibrary(guestBinaryPath);

            // Mark first byte as 'J' so we know we are running in hyperlight VM and not as real windows exe

            OS.VirtualProtect(loadAddress, (UIntPtr)(1024 * 4), OS.MemoryProtection.EXECUTE_READWRITE, out _);
            Marshal.WriteByte(loadAddress, (byte)'J');
            OS.VirtualProtect(loadAddress, (UIntPtr)(1024 * 4), OS.MemoryProtection.EXECUTE, out _);

            EntryPoint = (ulong)loadAddress + peInfo.EntryPointOffset;

            // Allocate 0x30001 for IO the additonal byte at the end is where the code would be loaded if we were running InProcess or under HyperVisor
            // The Guest will check this byte to see if it is null, if so it has been run from LoadLibrary and it will locate the code 
            // by looking at the address at pCodeOffset it then checks to ensure the code header is correct so it knows it is running in Hyperlight
            // Allows the guest to find the code if we are debugging 
            SourceAddress = OS.Allocate((IntPtr)0, Size);

            if (IntPtr.Zero == SourceAddress)
            {
                throw new ApplicationException("VirtualAlloc failed");
            }

            // Write a pointer to code so that guest exe can check that it is running in Hyperlight

            Marshal.WriteInt64(sandboxMemoryLayout.GetCodePointerAddress(SourceAddress), (long)loadAddress);
        }
        internal void LoadGuestBinaryIntoMemory(PEInfo peInfo)
        {
            sandboxMemoryLayout = new SandboxMemoryLayout(sandboxMemoryConfiguration, peInfo.Payload.Length, peInfo.StackReserve, peInfo.HeapReserve);
            Size = sandboxMemoryLayout.GetMemorySize();
            SourceAddress = OS.Allocate((IntPtr)0, Size);
            if (IntPtr.Zero == SourceAddress)
            {
                throw new ApplicationException("VirtualAlloc failed");
            }

            var hostCodeAddress = (ulong)SandboxMemoryLayout.GetHostCodeAddress(SourceAddress);
            // If we are running in memory the entry point will be relative to the sourceAddress if we are running in a Hypervisor it will be relative to 0x230000 which is where the code is loaded in the GP
            if (runFromProcessMemory)
            {
                EntryPoint = hostCodeAddress + peInfo.EntryPointOffset;
                Marshal.Copy(peInfo.Payload, 0, (IntPtr)hostCodeAddress, peInfo.Payload.Length);

                // When loading in memory we need to fix up the relocations in the exe to reflect the address the exe was loaded at.
                peInfo.PatchExeRelocations(hostCodeAddress);

                // Write a pointer to code so that guest exe can check that it is running in Hyperlight

                Marshal.WriteInt64(sandboxMemoryLayout.GetCodePointerAddress(SourceAddress), (long)hostCodeAddress);
            }
            else
            {
                EntryPoint = SandboxMemoryLayout.GuestCodeAddress + peInfo.EntryPointOffset;
                Marshal.Copy(peInfo.HyperVisorPayload, 0, (IntPtr)hostCodeAddress, peInfo.Payload.Length);

                // Write a pointer to code so that guest exe can check that it is running in Hyperlight

                Marshal.WriteInt64(sandboxMemoryLayout.GetCodePointerAddress(SourceAddress), (long)SandboxMemoryLayout.GuestCodeAddress);
            }

        }

        internal HyperlightPEB SetUpHyperLightPEB()
        {
            sandboxMemoryLayout.WriteMemoryLayout(SourceAddress, GetGuestAddressFromPointer(SourceAddress));
            var offset = GetAddressOffset();
            return new HyperlightPEB(sandboxMemoryLayout.GetFunctionDefinitionAddress(SourceAddress), sandboxMemoryConfiguration.HostFunctionDefinitionSize, offset);
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

            var pml4 = SandboxMemoryLayout.GetHostPML4Address(SourceAddress);
            var pdpt = SandboxMemoryLayout.GetHostPDPTAddress(SourceAddress);
            var pd = SandboxMemoryLayout.GetHostPDAddress(SourceAddress);

            Marshal.WriteInt64(pml4, 0, (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | SandboxMemoryLayout.PDPTGuestAddress));
            Marshal.WriteInt64(pdpt, 0, (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | (ulong)SandboxMemoryLayout.PDGuestAddress));

            for (var i = 0/*We do not map first 2 megs*/; i < 512; i++)
            {
                Marshal.WriteInt64(IntPtr.Add(pd, i * 8), ((i /*We map each VA to physical memory 2 megs lower*/) << 21) + (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | X64.PDE64_PS));
            }

            return rsp;
        }

        internal void SnapshotState()
        {

            //TODO: Track dirty pages instead of copying entire memory
            if (memorySnapShot == null)
            {
                memorySnapShot = new byte[Size];
            }
            Marshal.Copy(SourceAddress, memorySnapShot, 0, (int)Size);
        }
        internal void RestoreState()
        {
            Marshal.Copy(memorySnapShot!, 0, SourceAddress, (int)Size);
        }

        internal int GetReturnValue()
        {
            return Marshal.ReadInt32(sandboxMemoryLayout.GetOutputDataAddress(SourceAddress));
        }

        internal void SetOutBAddress(long pOutB)
        {
            var outBPointerAddress = sandboxMemoryLayout.GetOutBPointerAddress(SourceAddress);
            Marshal.WriteInt64(outBPointerAddress, pOutB);
        }

        internal (GuestErrorCode ErrorCode, string Message) GetGuestError()
        {
            var guestErrorAddress = sandboxMemoryLayout.GetGuestErrorAddress(SourceAddress);
            var error = Marshal.ReadInt64((IntPtr)guestErrorAddress);
            GuestErrorCode guestErrorCode = error switch
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
            return (ulong)Marshal.ReadInt64(sandboxMemoryLayout.GetDispatchFunctionPointerAddress(SourceAddress));
        }

        internal void WriteGuestFunctionCallDetails(string functionName, object[] args)
        {
            var headerSize = 0x08 + 0x08 + 0x08 * args.Length; // Pointer to function name, count of args, and arg list
            var stringTable = GetGuestCallStringTable(headerSize);
            var outputDataAddress = sandboxMemoryLayout.GetOutputDataAddress(SourceAddress);
            var pFunctionName = (long)stringTable.AddString(functionName);
            Marshal.WriteInt64(outputDataAddress, pFunctionName);
            Marshal.WriteInt64(outputDataAddress + 0x8, args.Length);
            for (var i = 0; i < args.Length; i++)
            {
                if (args[i].GetType() == typeof(int))
                {
                    Marshal.WriteInt64(outputDataAddress + 0x10 + 8 * i, (int)args[i]);
                }
                else if (args[i].GetType() == typeof(string))
                {
                    var addr = (long)(0x8000000000000000 | stringTable.AddString((string)args[i]));
                    Marshal.WriteInt64(outputDataAddress + 0x10 + 8 * i, addr);
                }
                else
                {
                    throw new ArgumentException("Unsupported parameter type");
                }
            }
        }

        SimpleStringTable GetGuestCallStringTable(int headerSize)
        {
            var outputDataAddress = sandboxMemoryLayout.GetOutputDataAddress(SourceAddress);
            return new SimpleStringTable(outputDataAddress + headerSize, sandboxMemoryConfiguration.OutputDataSize - headerSize, GetAddressOffset());
        }

        internal string GetHostCallMethodName()
        {
            var outputDataAddress = sandboxMemoryLayout.GetOutputDataAddress(SourceAddress);
            var strPtr = Marshal.ReadInt64((IntPtr)outputDataAddress);
            var methodName = Marshal.PtrToStringAnsi(GetHostAddressFromPointer(strPtr));
            ArgumentNullException.ThrowIfNull(methodName);
            return methodName;
        }

        internal object[] GetHostCallArgs(ParameterInfo[] parameters)
        {
            long strPtr;
            var args = new object[parameters.Length];
            var outputDataAddress = sandboxMemoryLayout.GetOutputDataAddress(SourceAddress);
            for (var i = 0; i < parameters.Length; i++)
            {
                if (parameters[i].ParameterType == typeof(int))
                {
                    args[i] = Marshal.ReadInt32(outputDataAddress + 8 * (i + 1));
                }
                else if (parameters[i].ParameterType == typeof(string))
                {
                    strPtr = Marshal.ReadInt64(outputDataAddress + 8 * (i + 1));
                    args[i] = Marshal.PtrToStringAnsi(GetHostAddressFromPointer(strPtr));
                }
                else
                {
                    throw new ArgumentException($"Unsupported parameter type: {parameters[i].ParameterType}");
                }
            }
            return args;
        }

        internal void WriteResponseFromHostMethodCall(int returnValue)
        {
            var inputDataAddress = sandboxMemoryLayout.GetInputDataAddress(SourceAddress);
            Marshal.WriteInt32(inputDataAddress, returnValue);
        }

        internal HyperlightException GetHostException()
        {
            var hostExceptionAddress = sandboxMemoryLayout.GetHostExceptionAddress(SourceAddress);
            HyperlightException hyperlightException = null;
            var dataLength = Marshal.ReadInt32(hostExceptionAddress);
            if (dataLength > 0)
            {
                var data = new byte[dataLength];
                Marshal.Copy(hostExceptionAddress + sizeof(int), data, 0, dataLength);
                var exceptionAsJson = Encoding.UTF8.GetString(data);
                // TODO: Switch to System.Text.Json - requires custom serialisation as default throws an exception when serialising if an inner exception is present
                // as it contains a Type: System.NotSupportedException: Serialization and deserialization of 'System.Type' instances are not supported and should be avoided since they can lead to security issues.
                // https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-converters-how-to?pivots=dotnet-6-0
                hyperlightException = JsonConvert.DeserializeObject<HyperlightException>(exceptionAsJson, new JsonSerializerSettings
                {
                    TypeNameHandling = TypeNameHandling.Auto
                });
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
            return runFromProcessMemory ? 0 : (long)SourceAddress - SandboxMemoryLayout.BaseAddress;
        }

        internal void WriteOutbException(Exception ex, ushort port)
        {
            var guestErrorAddress = sandboxMemoryLayout.GetGuestErrorAddress(SourceAddress);
            Marshal.WriteInt64(guestErrorAddress, (long)GuestErrorCode.OUTB_ERROR);

            var guestErrorMessagePointerAddress = sandboxMemoryLayout.GetGuestErrorMessagePointerAddress(SourceAddress);
            var guestErrorMessageAddress = GetHostAddressFromPointer(Marshal.ReadInt64(guestErrorMessagePointerAddress));
            var data = Encoding.UTF8.GetBytes($"Port:{port}, Message:{ex.Message}\0");
            if (data.Length <= sandboxMemoryConfiguration.GuestErrorMessageSize)
            {
                Marshal.Copy(data, 0, guestErrorMessageAddress, data.Length);
            }

            var hyperLightException = ex.GetType() == typeof(HyperlightException) ? ex as HyperlightException : new HyperlightException("OutB Error", ex);
            var hostExceptionPointer = sandboxMemoryLayout.GetHostExceptionAddress(SourceAddress);
            
            // TODO: Switch to System.Text.Json - requires custom serialisation as default throws an exception when serialising if an inner exception is present
            // as it contains a Type: System.NotSupportedException: Serialization and deserialization of 'System.Type' instances are not supported and should be avoided since they can lead to security issues.
            // https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-converters-how-to?pivots=dotnet-6-0
            var exceptionAsJson = JsonConvert.SerializeObject(hyperLightException, new JsonSerializerSettings
            {
                TypeNameHandling = TypeNameHandling.Auto
            });
            data = Encoding.UTF8.GetBytes(exceptionAsJson);
            var dataLength = data.Length;

            if (dataLength <= sandboxMemoryConfiguration.HostExceptionSize - sizeof(int))
            {
                Marshal.WriteInt32(hostExceptionPointer, dataLength);
                Marshal.Copy(data, 0, hostExceptionPointer + sizeof(int), data.Length);
            }

            // TODO: log that exception occurred.
        }
        internal ulong GetPebAddress()
        {
            if (runFromProcessMemory)
            {
                return sandboxMemoryLayout.GetInProcessPEBAddress(SourceAddress);
            }

            return (ulong)sandboxMemoryLayout.PEBAddress;
        }

        internal string ReadStringOutput()
        {
            var outputDataAddress = sandboxMemoryLayout.GetOutputDataAddress(SourceAddress);
            return Marshal.PtrToStringAnsi(outputDataAddress);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    // TODO: dispose managed state (managed objects)
                }

                if (IntPtr.Zero != SourceAddress)
                {
                    // TODO: check if this should take account of space used by loadlibrary.
                    OS.Free(SourceAddress, Size);
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
