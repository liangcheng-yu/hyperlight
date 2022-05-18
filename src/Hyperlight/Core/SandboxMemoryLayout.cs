using System;
using System.Runtime.InteropServices;

namespace Hyperlight.Core
{

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct HostFunctionDefinitions
    {
        internal ulong FunctionDefinitionSize;
        internal IntPtr FunctionDefinitions;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct HostExceptionData
    {
        internal ulong HostExceptionSize;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct GuestError
    {
        internal GuestErrorCode ErrorCode;
        internal ulong MaxMessageSize;
        internal IntPtr Message;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct CodeAndOutBPointers
    {
        internal IntPtr CodePointer;
        internal IntPtr OutBPointer;
    }

    internal class SandboxMemoryLayout
    {
        // TODO: update this to reflect dynamic nature of layout.

        // Address Space Layout - running in HyperVisor
        // Physical      Virtual
        // 0x00000000    0x00200000    PML4
        // 0x00001000    0x00201000    PDTP
        // 0x00002000    0x00202000    PD
        // 0x00003000    0x00203000    Function Definitions
        // 0x00004000    0x00204000    HostException - this contains any exception that occured in the host when called from the guest it allows the guest to signal an error occured and then have the exception rethrown when control returns to the host. 
        // 0x00005000    0x00205000    Guest Error - contains details of any errors that occured in the guest. 
        // 0x00005108    0x00205108    Pointer to code (this is used when code was loaded using loadlibrary so that the guest can check the code header to ensure it is running in HyperLight)
        // 0x00005110    0x00205110    Pointer to outb function (used when running in proc)
        // 0x00005120    0x00205120    64k for input data
        // 0x00015120    0x00215120    64k for output data
        // 0x00025120    0x00225120    Start of Code
        // 0x0012FFFF    0x0032FFFF    End of code (Start of code + 0x100000-1)
        // 0x009FFFFF    0x00BFFFFF    End of physical/max-Valid virtual
        // 0x00A00000    0x00C00000    Starting RSP

        // For our page table, we only mapped virtual memory up to 0x3FFFFFFF and map each 2 meg 
        // virtual chunk to physical addresses 2 megabytes below the virtual address.  Since we
        // map virtual up to 0x3FFFFFFF, the max physical address we handle is 0x3FDFFFFF (or 
        // 0x3FEF0000 physical total memory)

        const int PageTableSize = 0x3000;
        const int PDOffset = 0x2000;
        const int PDPTOffset = 0x1000;
        const int CodeOffSet = PageTableSize;

        public const int BaseAddress = 0x200000;
        public const int PML4GuestAddress = BaseAddress;
        public const ulong PDPTGuestAddress = BaseAddress + PDPTOffset;
        public const int PDGuestAddress = BaseAddress + PDOffset;
        public const ulong GuestCodeAddress = BaseAddress + CodeOffSet;

        readonly int pebOffset;
        readonly int inputDataOffset;
        readonly int outputDataOffset;
        readonly SandboxMemoryConfiguration sandboxMemoryConfiguration;
        readonly int codeSize;
        readonly int hostFunctionsOffset;
        readonly int hostExceptionOffset;
        readonly int errorMessageOffset;

        public ulong PEBAddress { get; init; }


        public IntPtr GetGuestErrorMessageAddress(IntPtr address) => IntPtr.Add(address, errorMessageOffset);
        public IntPtr GetGuestErrorAddress(IntPtr address) => address + PageTableSize + Marshal.SizeOf(typeof(HostFunctionDefinitions)) + Marshal.SizeOf(typeof(HostExceptionData)) + codeSize;
        public IntPtr GetGuestErrorMessageLengthAddress(IntPtr address) => GetGuestErrorAddress(address) + 8;
        public IntPtr GetGuestErrorMessagePointerAddress(IntPtr address) => GetGuestErrorAddress(address) + 16;
        public IntPtr GetFunctionDefinitionAddress(IntPtr address) => IntPtr.Add(address, hostFunctionsOffset);
        public IntPtr GetFunctionDefinitionLengthAddress(IntPtr address) => address + PageTableSize + codeSize;
        public IntPtr GetFunctionDefinitionPointerAddress(IntPtr address) => GetFunctionDefinitionLengthAddress(address) + 8;
        public IntPtr GetHostExceptionLengthAddress(IntPtr address) => address + PageTableSize + Marshal.SizeOf(typeof(HostFunctionDefinitions)) + codeSize; //Size is the first 8 bytes
        public IntPtr GetHostExceptionAddress(IntPtr address) => IntPtr.Add(address, hostExceptionOffset);
        public IntPtr GetOutBPointerAddress(IntPtr address) => address + inputDataOffset - 8; // OutB pointer is the 8 bytes before the input data buffer. 
        public IntPtr GetOutputDataAddress(IntPtr address) => address + outputDataOffset;
        public IntPtr GetInputDataAddress(IntPtr address) => address + inputDataOffset;
        public IntPtr GetCodePointerAddress(IntPtr address) => address + inputDataOffset - 16; // Code pointer is the 16 bytes before the input data buffer.
        public IntPtr GetDispatchFunctionPointerAddress(IntPtr address) => GetFunctionDefinitionAddress(address) + 8; // Pointer to Function Definitions is second element in Structure.
        public ulong GetInProcessPEBAddress(IntPtr address) => (ulong)(address + pebOffset);

        public static IntPtr GetHostPML4Address(IntPtr address) => address;
        public static IntPtr GetHostPDPTAddress(IntPtr address) => address + PDPTOffset;
        public static IntPtr GetHostPDAddress(IntPtr address) => address + PDOffset;
        public static IntPtr GetHostCodeAddress(IntPtr address) => address + CodeOffSet;

        internal SandboxMemoryLayout(SandboxMemoryConfiguration sandboxMemoryConfiguration, int codeSize)
        {
            this.sandboxMemoryConfiguration = sandboxMemoryConfiguration;
            this.codeSize = codeSize;
            pebOffset = PageTableSize + codeSize;
            inputDataOffset = PageTableSize + Marshal.SizeOf(typeof(HostFunctionDefinitions)) + Marshal.SizeOf(typeof(HostExceptionData)) + Marshal.SizeOf(typeof(GuestError)) + Marshal.SizeOf(typeof(CodeAndOutBPointers)) + codeSize;
            outputDataOffset = inputDataOffset + sandboxMemoryConfiguration.InputDataSize;
            PEBAddress = (ulong)(BaseAddress + pebOffset);
            hostFunctionsOffset = outputDataOffset + sandboxMemoryConfiguration.OutputDataSize;
            hostExceptionOffset = hostFunctionsOffset + sandboxMemoryConfiguration.HostFunctionDefinitionSize;
            errorMessageOffset = hostExceptionOffset + sandboxMemoryConfiguration.HostExceptionSize;
        }

        internal ulong GetMemorySize()
        {
            var totalMemory = codeSize + sandboxMemoryConfiguration.InputDataSize + sandboxMemoryConfiguration.OutputDataSize + sandboxMemoryConfiguration.MemoryBufferSize + sandboxMemoryConfiguration.StackSize + Marshal.SizeOf(typeof(HostFunctionDefinitions)) + sandboxMemoryConfiguration.HostExceptionSize + Marshal.SizeOf(typeof(HostExceptionData)) + Marshal.SizeOf(typeof(GuestError)) + sandboxMemoryConfiguration.GuestErrorMessageSize + PageTableSize + Marshal.SizeOf(typeof(CodeAndOutBPointers));

            // Size should be a multiple of 4K.

            var remainder = totalMemory % 0x1000;
            var multiples = totalMemory / 0x1000;
            return (ulong)(remainder > 0 ? (multiples + 1) * 0x1000 : totalMemory);
        }
    }
}
