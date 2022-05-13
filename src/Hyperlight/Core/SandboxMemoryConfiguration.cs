using System;

namespace Hyperlight.Core
{
    public class SandboxMemoryConfiguration
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


        const int DispatchPointerOffset = PageTableSize + 8;
        const int PageTableSize = 0x3000;
        const int PebOffset = PageTableSize;
        const int PointersSize = 16;
        const int PDOffset = 0x2000;
        const int PDPTOffset = 0x1000;
        const int GuestErrorMessagePrefixLength = 16; // 8 bytes for error code and 8 bytes for length from the Guest Error Structure Size

        public const int BaseAddress = 0x200000;
        public const int FunctionDefinitionOffset = PageTableSize;
        public const ulong PEBAddress = BaseAddress + PebOffset;
        public const int PML4GuestAddress = BaseAddress;
        public const ulong PDPTGuestAddress = BaseAddress + PDPTOffset;
        public const int PDGuestAddress = BaseAddress + PDOffset;

        /// <summary>
        /// GuestErrorSize defines the size of the memory buffer that is made available for Guest Error Details
        /// </summary>
        public int GuestErrorSize { get; }
        /// <summary>
        /// GuestErrorMessageSize defines the maximum size of the guest error message field.
        /// </summary>
        public int GuestErrorMessageSize { get { return GuestErrorSize - GuestErrorMessagePrefixLength; } }
        /// <summary>
        /// FunctionDefinitionSize defines the size of the memory buffer that is made available for Guest Function Definitions
        /// </summary>
        public int FunctionDefinitionSize { get; }
        /// <summary>
        /// HostExceptionSize defines the size of the memory buffer that is made available for serialising Host Exceptions
        /// </summary>
        public int HostExceptionSize { get; }
        /// <summary>
        /// InputDataSize defines the size of the memory buffer that is made available for input to the Guest Binary
        /// </summary>
        public int InputDataSize { get; }
        /// <summary>
        /// MemoryBufferSize defines the size of the memory buffer that is made available to the Guest Binary for dynamic allocation
        /// </summary>
        public int MemoryBufferSize { get; }
        /// <summary>
        /// OutputDataSize defines the size of the memory buffer that is made available for input to the Guest Binary
        /// </summary>
        public int OutputDataSize { get; }
        /// <summary>
        /// Stacksize defines the size of the stack made available to the Guest Binary 
        /// The Value used for StackSize should be used for the /GSnnnn and /STACK:nnnn,nnnn link parameters (cl.exe)
        /// when building a windows exe using msvc.
        /// see https://metricpanda.com/rival-fortress-update-45-dealing-with-__chkstk-__chkstk_ms-when-cross-compiling-for-windows/ for more information.
        /// </summary>
        public int StackSize { get; }

        public static SandboxMemoryConfiguration Default => new SandboxMemoryConfiguration(inputDataSize: 0x10000, outputDataSize: 0x10000, memoryBufferSize: 0x40000, stackSize: 0x100000, functionDefinitionSize: 0x1000, hostExceptionSize: 0x1000, guestErrorSize: 272);

        public SandboxMemoryConfiguration(int inputDataSize, int outputDataSize, int memoryBufferSize, int stackSize, int functionDefinitionSize, int hostExceptionSize, int guestErrorSize)
        {
            InputDataSize = inputDataSize;
            OutputDataSize = outputDataSize;
            MemoryBufferSize = memoryBufferSize;
            StackSize = stackSize;
            FunctionDefinitionSize = functionDefinitionSize;
            HostExceptionSize = hostExceptionSize;
            GuestErrorSize = guestErrorSize;
        }

        int codeOffSet => OutputDataOffset + OutputDataSize;
        int InputDataOffset => PageTableSize + FunctionDefinitionSize + HostExceptionSize + GuestErrorSize + PointersSize;
        int OutputDataOffset => InputDataOffset + InputDataSize;
        
        public ulong GuestCodeAddress => BaseAddress + (ulong)codeOffSet;
        public int TotalSize => InputDataSize + OutputDataSize + MemoryBufferSize + StackSize + FunctionDefinitionSize + HostExceptionSize + GuestErrorSize + PageTableSize + PointersSize;

        public IntPtr GetHostExceptionAddress(IntPtr address) => address + PageTableSize + FunctionDefinitionSize;
        public IntPtr GetGuestErrorAddress(IntPtr address) => address + PageTableSize + FunctionDefinitionSize + HostExceptionSize;
        public IntPtr GetOutBPointerAddress(IntPtr address) => address + InputDataOffset - 8; // OutB pointer is the 8 bytes before the input data buffer. 
        public IntPtr GetOutputDataAddress(IntPtr address) => address + OutputDataOffset;
        public IntPtr GetInputDataAddress(IntPtr address) => address + InputDataOffset;
        public IntPtr GetCodePointerAddress(IntPtr address) => address + InputDataOffset - 16; // OutB pointer is the 16 bytes before the input data buffer.
        public IntPtr GetHostCodeAddress(IntPtr address) => address + codeOffSet;
        public static IntPtr GetHostPML4Address(IntPtr address) => address;
        public static IntPtr GetHostPDPTAddress(IntPtr address) => address + PDPTOffset;
        public static IntPtr GetHostPDAddress(IntPtr address) => address + PDOffset;
        public static IntPtr GetDispatchFunctionPointerAddress(IntPtr address) => address + DispatchPointerOffset;
        public static ulong GetInProcessPEBAddress(IntPtr address) => (ulong)address + PebOffset;
    }
}
