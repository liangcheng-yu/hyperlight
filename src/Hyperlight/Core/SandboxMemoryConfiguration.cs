namespace Hyperlight.Core
{
    public class SandboxMemoryConfiguration
    {
        const int DefaultInputSize = 0x10000;
        const int DefaultOutputSize = 0x10000;
        const int DefaultMemoryBufferSize = 0x40000;
        const int DefaultStackSize = 0x100000;
        const int DefaultFunctionDefinitionSize = 0x1000;
        const int DefaultHostExceptionSize = 0x1000;
        const int DefaultGuestErrorMessageSize = 0x100;

        /// <summary>
        /// GuestErrorMessageSize defines the maximum size of the guest error message field.
        /// </summary>
        public int GuestErrorMessageSize { get; }
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

        public SandboxMemoryConfiguration(int inputDataSize = DefaultInputSize, int outputDataSize = DefaultOutputSize, int memoryBufferSize = DefaultMemoryBufferSize, int stackSize = DefaultStackSize, int functionDefinitionSize = DefaultFunctionDefinitionSize, int hostExceptionSize = DefaultHostExceptionSize, int guestErrorMessageSize = DefaultGuestErrorMessageSize)
        {
            InputDataSize = inputDataSize;
            OutputDataSize = outputDataSize;
            MemoryBufferSize = memoryBufferSize;
            StackSize = stackSize;
            FunctionDefinitionSize = functionDefinitionSize;
            HostExceptionSize = hostExceptionSize;
            GuestErrorMessageSize = guestErrorMessageSize;
        }
    }
}
