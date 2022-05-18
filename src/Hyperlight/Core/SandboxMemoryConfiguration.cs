namespace Hyperlight.Core
{
    public class SandboxMemoryConfiguration
    {
        const int DefaultInputSize = 0x10000;
        const int DefaultOutputSize = 0x10000;
        const int DefaultMemoryBufferSize = 0x40000;
        const int DefaultStackSize = 0x100000;
        const int DefaultHostFunctionDefinitionSize = 0x1000;
        const int DefaultHostExceptionSize = 0x1000;
        const int DefaultGuestErrorMessageSize = 0x100;

        const int MinInputSize = 0x10000;
        const int MinOutputSize = 0x10000;
        const int MinMemoryBufferSize = 0x40000;
        const int MinStackSize = 0x100000;
        const int MinHostFunctionDefinitionSize = 0x400;
        const int MinHostExceptionSize = 0x400;
        const int MinGuestErrorMessageSize = 0x100;

        /// <summary>
        /// GuestErrorMessageSize defines the maximum size of the guest error message field.
        /// </summary>
        public int GuestErrorMessageSize { get; }
        /// <summary>
        /// FunctionDefinitionSize defines the size of the memory buffer that is made available for Guest Function Definitions
        /// </summary>
        public int HostFunctionDefinitionSize { get; }
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

        public SandboxMemoryConfiguration(int inputDataSize = DefaultInputSize, int outputDataSize = DefaultOutputSize, int memoryBufferSize = DefaultMemoryBufferSize, int stackSize = DefaultStackSize, int functionDefinitionSize = DefaultHostFunctionDefinitionSize, int hostExceptionSize = DefaultHostExceptionSize, int guestErrorMessageSize = DefaultGuestErrorMessageSize)
        {
            InputDataSize = inputDataSize < MinInputSize ? MinInputSize : inputDataSize;
            OutputDataSize = outputDataSize < MinOutputSize ? MinOutputSize : outputDataSize;
            MemoryBufferSize = memoryBufferSize < MinMemoryBufferSize ? MinMemoryBufferSize : memoryBufferSize;
            StackSize = stackSize < MinStackSize ? MinStackSize : stackSize;
            HostFunctionDefinitionSize = functionDefinitionSize < MinHostFunctionDefinitionSize ? MinHostFunctionDefinitionSize : functionDefinitionSize;
            HostExceptionSize = hostExceptionSize < MinHostExceptionSize ? MinHostExceptionSize : hostExceptionSize;
            GuestErrorMessageSize = guestErrorMessageSize < MinGuestErrorMessageSize ? MinGuestErrorMessageSize : guestErrorMessageSize;
        }
    }
}
