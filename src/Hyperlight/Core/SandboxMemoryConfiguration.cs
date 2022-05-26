using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;

namespace Hyperlight.Core
{
    public class SandboxMemoryConfiguration
    {
        const int DefaultInputSize = 0x4000;
        const int DefaultOutputSize = 0x4000;
        const int DefaultHostFunctionDefinitionSize = 0x1000;
        const int DefaultHostExceptionSize = 0x1000;
        const int DefaultGuestErrorMessageSize = 0x100;

        const int MinInputSize = 0x2000;
        const int MinOutputSize = 0x2000;
        const int MinHostFunctionDefinitionSize = 0x400;
        const int MinHostExceptionSize = 0x400;
        const int MinGuestErrorMessageSize = 0x80;

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
        /// OutputDataSize defines the size of the memory buffer that is made available for input to the Guest Binary
        /// </summary>
        public int OutputDataSize { get; }

        public SandboxMemoryConfiguration(int inputDataSize = DefaultInputSize, int outputDataSize = DefaultOutputSize, int functionDefinitionSize = DefaultHostFunctionDefinitionSize, int hostExceptionSize = DefaultHostExceptionSize, int guestErrorMessageSize = DefaultGuestErrorMessageSize)
        {
            InputDataSize = inputDataSize < MinInputSize ? MinInputSize : inputDataSize;
            OutputDataSize = outputDataSize < MinOutputSize ? MinOutputSize : outputDataSize;
            HostFunctionDefinitionSize = functionDefinitionSize < MinHostFunctionDefinitionSize ? MinHostFunctionDefinitionSize : functionDefinitionSize;
            HostExceptionSize = hostExceptionSize < MinHostExceptionSize ? MinHostExceptionSize : hostExceptionSize;
            GuestErrorMessageSize = guestErrorMessageSize < MinGuestErrorMessageSize ? MinGuestErrorMessageSize : guestErrorMessageSize;
        }
    }
}
