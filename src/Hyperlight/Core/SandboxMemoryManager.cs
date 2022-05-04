using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Native;
using System.Text;
using Newtonsoft.Json;

namespace Hyperlight.Core
{


    // Address Space Layout - Assume 10 meg physical (0xA00000) memory and 1 meg code (0x100000)
    // Physical      Virtual
    // 0x00000000    0x00200000    Start of physical/min-Valid virtual
    // 0x00001000    0x00201000    PML4
    // 0x00002000    0x00202000    PDTP
    // 0x00003000    0x00203000    PD
    // 0x00004000    0x00204000    Function Definitions
    // 0x0000EEE0    0x0020EEE0    HostException - this contains any exception that occured in the host when called from the guest it allows the guest to signal an error occured and then have the exception rethrown when control returns to the host. 
    // 0x0000FEE0    0x0020FEE0    Guest Error - contains details of any errors that occured in the guest. 
    // 0x0000FFE8    0x0020FFE8    Pointer to code (this is used when code was loaded using loadlibrary so that the guest can check the code header to ensure it is running in HyperLight)
    // 0x0000FFF0    0x0020FFF0    Pointer to outb function (used when running in proc)
    // 0x00010000    0x00210000    64k for input data
    // 0x00020000    0x00220000    64k for output data
    // 0x00030000    0x00230000    Start of Code
    // 0x0012FFFF    0x0032FFFF    End of code (Start of code + 0x100000-1)
    // 0x009FFFFF    0x00BFFFFF    End of physical/max-Valid virtual
    // 0x00A00000    0x00C00000    Starting RSP

    // Address Space Layout - Assume max 0x3FE00000 physical memory and 1 meg code (0x100000)
    // Physical      Virtual
    // 0x00000000    0x00200000    Start of physical/min-Valid virtual
    // 0x00001000    0x00201000    PML4
    // 0x00002000    0x00202000    PDTP
    // 0x00003000    0x00203000    PD
    // 0x00004000    0x00204000    Function Definitions
    // 0x0000EEE0    0x0020EEE0    HostException - this contains any exception that occured in the host when called from the guest it allows the guest to signal an error occured and then have the exception rethrown when control returns to the host. 
    // 0x0000FEE0    0x0020FEE0    Guest Error - contains details of any errors that occured in the guest. 
    // 0x0000FFE8    0x0020FFE8    Pointer to code (this is used when code was loaded using loadlibrary so that the guest can check the code header to ensure it is running in HyperLight)
    // 0x0000FFF0    0x0020FFF0    Pointer to outb function (used when running in proc)
    // 0x00010000    0x00210000    64k for input data
    // 0x00020000    0x00220000    64k for output data
    // 0x00030000    0x00230000    Start of Code
    // 0x0012FFFF    0x0032FFFF    End of code (Start of code + 0x100000-1)
    // 0x3FDFFFFF    0x3FFFFFFF    End of physical/max-Valid virtual
    // 0x3FE00000    0x40000000    Starting RSP

    // For our page table, we only mapped virtual memory up to 0x3FFFFFFF and map each 2 meg 
    // virtual chunk to physical addresses 2 megabytes below the virtual address.  Since we
    // map virtual up to 0x3FFFFFFF, the max physical address we handle is 0x3FDFFFFF (or 
    // 0x3FEF0000 physical total memory)

    internal class SandboxMemoryManager : IDisposable
    {

        public static readonly IntPtr BaseAddress = (IntPtr)0x200000;
        public static readonly int Pml4_addr = (int)BaseAddress;
        public static readonly ulong CodeAddress = (ulong)BaseAddress + CodeOffset;

        static readonly IntPtr codeAddress = BaseAddress + CodeOffset;
        static readonly int pdpt_addr = (int)BaseAddress + 0x1000;
        static readonly int pd_addr = (int)BaseAddress + 0x2000;

        const int CodeOffset = 0x25120;
        const int DispatchPointerOffset = 0x3008;
        const int FunctionDefinitionLength = 0x1000;
        const int FunctionDefinitionOffset = 0x3000;
        const int GuestErrorOffset = 0x5000;
        const int HostExceptionOffset = 0x4000;
        const int HostExceptionSize = 4096;
        const int InputDataOffset = 0x5120;
        const int OutputDataOffset = 0x15120;
        const int PCodeOffset = InputDataOffset - 24;
        const int PebOffset = 0x3000;
        const int POutBOffset = InputDataOffset - 16;

        public IntPtr SourceAddress { get; private set; }
        public ulong EntryPoint { get; private set; }

        private bool disposedValue;
        IntPtr loadAddress = IntPtr.Zero;
        byte[] memorySnapShot;

        readonly bool runFromProcessMemory;
        readonly ulong size;

        internal SandboxMemoryManager(ulong size, bool runFromProcessMemory = false)
        {
            this.size = size;
            this.runFromProcessMemory = runFromProcessMemory;
        }

        internal void LoadGuestBinaryUsingLoadLibrary(string guestBinaryPath, PEInfo peInfo)
        {
            ArgumentNullException.ThrowIfNull(guestBinaryPath, nameof(guestBinaryPath));
            ArgumentNullException.ThrowIfNull(peInfo, nameof(peInfo));

            loadAddress = OS.LoadLibrary(guestBinaryPath);

            // Mark first byte as 'J' so we know we are running in hyperlight VM and not as real windows exe
            // TODO: protect memory again after modification
            OS.VirtualProtect(loadAddress, (UIntPtr)(1024 * 4), OS.MemoryProtection.EXECUTE_READWRITE, out _);
            Marshal.WriteByte(loadAddress, (byte)'J');
            var e_lfanew = Marshal.ReadInt32(loadAddress + 0x3C);

            EntryPoint = (ulong)loadAddress + peInfo.EntryPointOffset;

            // Allocate 0x30001 for IO the additonal byte at the end is where the code would be loaded if we were running InProcess or under HyperVisor
            // The Guest will check this byte to see if it is null, if so it has been run from LoadLibrary and it will locate the code 
            // by looking at the address at pCodeOffset it then checks to ensure the code header is correct so it knows it is running in Hyperlight
            // Allows the guest to find the code if we are debugging 
            SourceAddress = OS.Allocate((IntPtr)0, (ulong)CodeOffset + 1);

            if (IntPtr.Zero == SourceAddress)
            {
                throw new ApplicationException("VirtualAlloc failed");
            }

            // Write a pointer to code so that guest exe can check that it is running in Hyperlight

            Marshal.WriteInt64(SourceAddress + PCodeOffset, (long)loadAddress);
        }
        internal void LoadGuestBinaryIntoMemory(PEInfo peInfo)
        {

            SourceAddress = OS.Allocate((IntPtr)0, size);
            if (IntPtr.Zero == SourceAddress)
            {
                throw new ApplicationException("VirtualAlloc failed");
            }

            // If we are running in memory the entry point will be relative to the sourceAddress if we are running in a Hypervisor it will be relative to 0x230000 which is where the code is loaded in the GP
            if (runFromProcessMemory)
            {
                EntryPoint = (ulong)SourceAddress + (ulong)CodeOffset + peInfo.EntryPointOffset;
                Marshal.Copy(peInfo.Payload, 0, SourceAddress + CodeOffset, peInfo.Payload.Length);

                // When loading in memory we need to fix up the relocations in the exe to reflect the address the exe was loaded at.
                peInfo.PatchExeRelocations((ulong)SourceAddress + (ulong)CodeOffset);
            }
            else
            {
                EntryPoint = (ulong)codeAddress + peInfo.EntryPointOffset;
                Marshal.Copy(peInfo.HyperVisorPayload, 0, SourceAddress + CodeOffset, peInfo.Payload.Length);
            }
        }

        internal HyperlightPEB SetUpHyperLightPEB()
        {
            ulong offset = 0;
            if (!runFromProcessMemory)
            {
                offset = (ulong)SourceAddress - (ulong)BaseAddress;
            }

            return new HyperlightPEB(IntPtr.Add(SourceAddress, FunctionDefinitionOffset), FunctionDefinitionLength, offset);
        }

        internal ulong SetUpHyperVisorPartition()
        {
            ulong rsp = size + (ulong)BaseAddress; // Add 0x200000 because that's the start of mapped memory

            // For MSVC, move rsp down by 0x28.  This gives the called 'main' function the appearance that rsp was
            // was 16 byte aligned before the 'call' that calls main (note we don't really have a return value on the
            // stack but some assembly instructions are expecting rsp have started 0x8 bytes off of 16 byte alignment
            // when 'main' is invoked.  We do 0x28 instead of 0x8 because MSVC can expect that there are 0x20 bytes
            // of space to write to by the called function.  I am not sure if this happens with the 'main' method, but
            // we do this just in case.
            // NOTE: We do this also for GCC freestanding binaries because we specify __attribute__((ms_abi)) on the start method
            rsp -= 0x28;

            // Create pagetable

            var pml4 = IntPtr.Add(SourceAddress, Pml4_addr - (int)BaseAddress);
            var pdpt = IntPtr.Add(SourceAddress, pdpt_addr - (int)BaseAddress);
            var pd = IntPtr.Add(SourceAddress, pd_addr - (int)BaseAddress);

            Marshal.WriteInt64(pml4, 0, (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | (ulong)pdpt_addr));
            Marshal.WriteInt64(pdpt, 0, (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | (ulong)pd_addr));

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
                memorySnapShot = new byte[size];
            }
            Marshal.Copy(SourceAddress, memorySnapShot, 0, (int)size);
        }
        internal void RestoreState()
        {
            Marshal.Copy(memorySnapShot!, 0, SourceAddress, (int)size);
        }

        internal int GetReturnValue()
        {
            return Marshal.ReadInt32(SourceAddress + OutputDataOffset);
        }

        internal void SetOutBAddress(long pOutB)
        {
            Marshal.WriteInt64(SourceAddress + POutBOffset, pOutB);
        }

        internal GuestError GetGuestError()
        {
            var guestErrorAddress = SourceAddress + GuestErrorOffset;
            return Marshal.PtrToStructure<GuestError>(guestErrorAddress);
        }

        internal ulong GetPointerToDispatchFunction()
        {
            var dispatchFunctionAddress = SourceAddress + DispatchPointerOffset;
            return (ulong)Marshal.ReadInt64(dispatchFunctionAddress);
        }

        internal void WriteGuestFunctionCallDetails(string functionName, object[] args)
        {
            var headerSize = 0x08 + 0x08 + 0x08 * args.Length; // Pointer to function name, count of args, and arg list
            var stringTable = GetGuestCallStringTable(headerSize);
            var outputDataAddress = SourceAddress + OutputDataOffset;
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
            ulong offset = 0;
            if (!runFromProcessMemory)
            {
                offset = (ulong)SourceAddress - (ulong)BaseAddress;
            }
            var outputDataAddress = SourceAddress + OutputDataOffset;
            return new SimpleStringTable(outputDataAddress + headerSize, InputDataOffset - headerSize, offset);
        }

        internal string GetHostCallMethodName()
        {
            // Offset contains the adjustment that needs to be made to addresses when running in Hypervisor so that the address reflects the host or guest address correctly
            ulong offset = 0;
            if (!runFromProcessMemory)
            {
                offset = (ulong)SourceAddress - (ulong)BaseAddress;
            }
            var outputDataAddress = SourceAddress + OutputDataOffset;
            var strPtr = Marshal.ReadInt64((IntPtr)outputDataAddress);
            var methodName = Marshal.PtrToStringAnsi((IntPtr)((ulong)strPtr + offset));
            ArgumentNullException.ThrowIfNull(methodName);
            return methodName;
        }

        internal object[] GetHostCallArgs(ParameterInfo[] parameters)
        {
            // Offset contains the adjustment that needs to be made to addresses when running in Hypervisor so that the address reflects the host or guest address correctly
            ulong offset = 0;
            if (!runFromProcessMemory)
            {
                offset = (ulong)SourceAddress - (ulong)BaseAddress;
            }
            long strPtr;
            var args = new object[parameters.Length];
            var outputDataAddress = SourceAddress + OutputDataOffset;
            for (var i = 0; i < parameters.Length; i++)
            {
                if (parameters[i].ParameterType == typeof(int))
                {
                    args[i] = Marshal.ReadInt32(outputDataAddress + 8 * (i + 1));
                }
                else if (parameters[i].ParameterType == typeof(string))
                {
                    strPtr = Marshal.ReadInt64(outputDataAddress + 8 * (i + 1));
                    args[i] = Marshal.PtrToStringAnsi((IntPtr)((ulong)strPtr + offset));
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
            Marshal.WriteInt32(SourceAddress + InputDataOffset, returnValue);
        }

        internal HyperlightException GetHostException()
        {
            var hostExceptionPointer = SourceAddress + HostExceptionOffset;
            HyperlightException hyperlightException = null;
            var dataLength = Marshal.ReadInt32(hostExceptionPointer);
            var data = new byte[dataLength];
            if (dataLength > 0)
            {
                Marshal.Copy(hostExceptionPointer + sizeof(int), data, 0, dataLength);
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

        internal void WriteOutbException(Exception ex, ushort port)
        {
            var guestError = new GuestError() { ErrorCode = GuestErrorCode.OUTB_ERROR, Message = $"Port:{port}, Message:{ex.Message}" };
            Marshal.StructureToPtr<GuestError>(guestError, SourceAddress + GuestErrorOffset, false);
            var hyperLightException = ex.GetType() == typeof(HyperlightException) ? ex as HyperlightException : new HyperlightException("OutB Error", ex);
            var hostExceptionPointer = SourceAddress + HostExceptionOffset;
            // TODO: Switch to System.Text.Json - requires custom serialisation as default throws an exception when serialising if an inner exception is present
            // as it contains a Type: System.NotSupportedException: Serialization and deserialization of 'System.Type' instances are not supported and should be avoided since they can lead to security issues.
            // https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-converters-how-to?pivots=dotnet-6-0
            var exceptionAsJson = JsonConvert.SerializeObject(hyperLightException, new JsonSerializerSettings
            {
                TypeNameHandling = TypeNameHandling.Auto
            });
            var data = Encoding.UTF8.GetBytes(exceptionAsJson);
            var dataLength = data.Length;
            // TODO: Currently the size of the serialised exception is fixed , once it is dynamic remove this
            if (dataLength <= HostExceptionSize - sizeof(int))
            {
                Marshal.WriteInt32(hostExceptionPointer, dataLength);
                Marshal.Copy(data, 0, hostExceptionPointer + sizeof(int), data.Length);
            }
        }
        internal ulong GetPebAddress()
        {
            if (runFromProcessMemory)
            {
                return (ulong)SourceAddress + PebOffset;
            }

            return (ulong) BaseAddress + PebOffset;
        }

        internal string ReadStringOutput()
        {
            return Marshal.PtrToStringAnsi(SourceAddress + OutputDataOffset);
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
                    OS.Free(SourceAddress, size);
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
