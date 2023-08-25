using System;
using System.Runtime.InteropServices;
using System.Text;
namespace Hyperlight.Core
{
    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    // Override equals and operator equals on value types
#pragma warning disable CA1815
    public readonly struct SandboxMemoryConfiguration
    // Override equals and operator equals on value types
#pragma warning restore CA1815
    {
        // NOTE: do not change the order of these struct fields without
        // also changing the fields in src/hyperlight_capi/src/mem/config.rs
        // to match

        /// <summary>
        /// defines the maximum size of the guest error message field.
        /// </summary>
        public ulong GuestErrorBufferSize { get; init; }

        /// <summary>
        /// defines the size of the memory buffer that is made available for Guest Function Definitions
        /// </summary>
        public ulong HostFunctionDefinitionSize { get; init; }

        /// <summary>
        /// defines the size of the memory buffer that is made available for serialising Host Exceptions
        /// </summary>
        public ulong HostExceptionSize { get; init; }

        /// <summary>
        /// defines the size of the memory buffer that is made available for input to the Guest Binary
        /// </summary>        
        public ulong InputDataSize { get; init; }

        /// <summary>
        /// defines the size of the memory buffer that is made available for input to the Guest Binary
        /// </summary>
        public ulong OutputDataSize { get; init; }

        /// <summary>
        /// defines the stack size to be allocated for the guest.
        /// if set to 0 or not defined, the stack size will be determined
        /// from the guest executable's PE file header
        /// </summary>
        public ulong StackSizeOverride { get; init; }

        /// <summary>
        /// defines the heap size to be allocated for the guest.
        /// if set to 0 or not defined, the heap size will be determined
        /// from the guest executable's PE file header
        /// </summary>
        public ulong HeapSizeOverride { get; init; }

        /// <summary>
        /// Create a new SandboxMemoryConfiguration, with default
        /// values.
        /// 
        /// NOTE: this parameter-less constructor is necessary so C#
        /// doesn't auto-implement a param-free ctor that assigns
        /// 0 to all fields.
        /// </summary>
        public SandboxMemoryConfiguration()
        {
            this = mem_config_default();
        }

        /// <summary>
        /// Create a new SandboxMemoryConfiguration with the given parameters
        /// </summary>
        /// <param name="inputDataSize">
        /// The size in guest memory to reserve for input data
        /// </param>
        /// <param name="outputDataSize">
        /// The size in guest memory to reserve for output data
        /// </param>
        /// <param name="hostFunctionDefinitionSize">
        /// The size in guest memory to reserve for host functions
        /// </param>
        /// <param name="hostExceptionSize">
        /// The size in guest memory to reserve for host exceptions
        /// </param>
        /// <param name="guestErrorMessageSize">
        /// The size in guest memory to reserve for guest errors
        /// </param>
        public SandboxMemoryConfiguration(
            ulong inputDataSize,
            ulong outputDataSize,
            ulong hostFunctionDefinitionSize,
            ulong hostExceptionSize,
            ulong guestErrorMessageSize,
            ulong stackSizeOverride = 0,
            ulong heapSizeOverride = 0
        )
        {
            var config = mem_config_new(
                inputDataSize,
                outputDataSize,
                hostFunctionDefinitionSize,
                hostExceptionSize,
                guestErrorMessageSize,
                stackSizeOverride,
                heapSizeOverride
            );
            this.GuestErrorBufferSize = config.GuestErrorBufferSize;
            this.HostFunctionDefinitionSize = config.HostFunctionDefinitionSize;
            this.HostExceptionSize = config.HostExceptionSize;
            this.InputDataSize = config.InputDataSize;
            this.OutputDataSize = config.OutputDataSize;
            this.HeapSizeOverride = config.HeapSizeOverride;
            this.StackSizeOverride = config.StackSizeOverride;
        }

        public SandboxMemoryConfiguration WithInputDataSize(ulong size)
        {
            return new SandboxMemoryConfiguration()
            {
                InputDataSize = size,
                OutputDataSize = this.OutputDataSize,
                HostFunctionDefinitionSize = this.HostFunctionDefinitionSize,
                HostExceptionSize = this.HostExceptionSize,
                GuestErrorBufferSize = this.GuestErrorBufferSize,
                HeapSizeOverride = this.HeapSizeOverride,
                StackSizeOverride = this.StackSizeOverride
            };
        }

        public SandboxMemoryConfiguration WithOutputDataSize(ulong size)
        {
            return new SandboxMemoryConfiguration()
            {
                InputDataSize = this.InputDataSize,
                OutputDataSize = size,
                HostFunctionDefinitionSize = this.HostFunctionDefinitionSize,
                HostExceptionSize = this.HostExceptionSize,
                GuestErrorBufferSize = this.GuestErrorBufferSize,
                HeapSizeOverride = this.HeapSizeOverride,
                StackSizeOverride = this.StackSizeOverride
            };
        }

        public SandboxMemoryConfiguration WithHostFunctionDefinitionSize(ulong size)
        {
            return new SandboxMemoryConfiguration()
            {
                InputDataSize = this.InputDataSize,
                OutputDataSize = this.OutputDataSize,
                HostFunctionDefinitionSize = size,
                HostExceptionSize = this.HostExceptionSize,
                GuestErrorBufferSize = this.GuestErrorBufferSize,
                HeapSizeOverride = this.HeapSizeOverride,
                StackSizeOverride = this.StackSizeOverride
            };
        }

        public SandboxMemoryConfiguration WithHostExceptionSize(ulong size)
        {
            return new SandboxMemoryConfiguration()
            {
                InputDataSize = this.InputDataSize,
                OutputDataSize = this.OutputDataSize,
                HostFunctionDefinitionSize = this.HostFunctionDefinitionSize,
                HostExceptionSize = size,
                GuestErrorBufferSize = this.GuestErrorBufferSize,
                HeapSizeOverride = this.HeapSizeOverride,
                StackSizeOverride = this.StackSizeOverride
            };
        }

        public SandboxMemoryConfiguration WithGuestErrorBufferSize(ulong size)
        {
            return new SandboxMemoryConfiguration()
            {
                InputDataSize = this.InputDataSize,
                OutputDataSize = this.OutputDataSize,
                HostFunctionDefinitionSize = this.HostFunctionDefinitionSize,
                HostExceptionSize = this.HostExceptionSize,
                GuestErrorBufferSize = size,
                HeapSizeOverride = this.HeapSizeOverride,
                StackSizeOverride = this.StackSizeOverride
            };
        }

        public SandboxMemoryConfiguration WithHeapSizeOverride(ulong size)
        {
            return new SandboxMemoryConfiguration()
            {
                InputDataSize = this.InputDataSize,
                OutputDataSize = this.OutputDataSize,
                HostFunctionDefinitionSize = this.HostFunctionDefinitionSize,
                HostExceptionSize = this.HostExceptionSize,
                GuestErrorBufferSize = this.GuestErrorBufferSize,
                HeapSizeOverride = size,
                StackSizeOverride = this.StackSizeOverride
            };
        }

        public SandboxMemoryConfiguration WithStackSizeOverride(ulong size)
        {
            return new SandboxMemoryConfiguration()
            {
                InputDataSize = this.InputDataSize,
                OutputDataSize = this.OutputDataSize,
                HostFunctionDefinitionSize = this.HostFunctionDefinitionSize,
                HostExceptionSize = this.HostExceptionSize,
                GuestErrorBufferSize = this.GuestErrorBufferSize,
                HeapSizeOverride = this.HeapSizeOverride,
                StackSizeOverride = size
            };
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern SandboxMemoryConfiguration mem_config_new(
            ulong inputSize,
            ulong outputSize,
            ulong hostFunctionDefinitionSize,
            ulong hostExceptionSize,
            ulong guestErrorBufferSize,
            ulong stackSizeOverride,
            ulong heapSizeOverride
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern SandboxMemoryConfiguration mem_config_default();

#pragma warning restore CA1707 // Remove the underscores from member name
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory



    }
}
