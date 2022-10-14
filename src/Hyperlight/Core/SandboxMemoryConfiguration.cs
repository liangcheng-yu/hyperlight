using System;
using System.Runtime.InteropServices;
using Hyperlight.Wrapper;
namespace Hyperlight.Core
{
    public class SandboxMemoryConfiguration : IDisposable
    {
        const ulong DefaultInputSize = 0x4000;
        const ulong DefaultOutputSize = 0x4000;
        const ulong DefaultHostFunctionDefinitionSize = 0x1000;
        const ulong DefaultHostExceptionSize = 0x1000;
        const ulong DefaultGuestErrorMessageSize = 0x100;

        private Wrapper.Context ctxWrapper;
        private Wrapper.Handle hdlWrapper;
        private bool disposed;

        public SandboxMemoryConfiguration(
            Wrapper.Context ctx,
            ulong inputDataSize = DefaultInputSize,
            ulong outputDataSize = DefaultOutputSize,
            ulong functionDefinitionSize = DefaultHostFunctionDefinitionSize,
            ulong hostExceptionSize = DefaultHostExceptionSize,
            ulong guestErrorMessageSize = DefaultGuestErrorMessageSize
        )
        {
            HyperlightException.ThrowIfNull(ctx, Sandbox.CorrelationId.Value!, GetType().Name);
            var rawHandle = mem_config_new(
                ctx.ctx,
                inputDataSize,
                outputDataSize,
                functionDefinitionSize,
                hostExceptionSize,
                guestErrorMessageSize
            );
            this.ctxWrapper = ctx;
            this.hdlWrapper = new Wrapper.Handle(ctx, rawHandle);
        }

        public void Dispose()
        {
            this.Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!this.disposed)
            {
                if (disposing)
                {
                    this.hdlWrapper.Dispose();
                }
                this.disposed = true;
                HyperlightLogger.LogError("Disposed", Sandbox.CorrelationId.Value!, GetType().Name);
            }
        }

        /// <summary>
        /// GuestErrorMessageSize defines the maximum size of the guest error message field.
        /// </summary>
        public ulong GuestErrorMessageSize
        {
            get
            {
                return mem_config_get_guest_error_message_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                mem_config_set_guest_error_message_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle,
                    value
                );
            }
        }
        /// <summary>
        /// FunctionDefinitionSize defines the size of the memory buffer that is made available for Guest Function Definitions
        /// </summary>
        public ulong HostFunctionDefinitionSize
        {
            get
            {
                return mem_config_get_host_function_definition_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                mem_config_set_host_function_definition_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle,
                    value
                );
            }
        }
        /// <summary>
        /// HostExceptionSize defines the size of the memory buffer that is made available for serialising Host Exceptions
        /// </summary>
        public ulong HostExceptionSize
        {
            get
            {
                return mem_config_get_host_exception_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                mem_config_set_host_exception_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle,
                    value
                );
            }
        }

        /// <summary>
        /// InputDataSize defines the size of the memory buffer that is made available for input to the Guest Binary
        /// </summary>
        public ulong InputDataSize
        {
            get
            {
                return mem_config_get_input_data_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                mem_config_set_input_data_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle,
                    value
                );
            }
        }

        /// <summary>
        /// OutputDataSize defines the size of the memory buffer that is made available for input to the Guest Binary
        /// </summary>
        public ulong OutputDataSize
        {
            get
            {
                return mem_config_get_output_data_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                mem_config_set_output_data_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle,
                    value
                );
            }
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA1401 // P/Invoke method should not be visible
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        /////
        // start memory configuration
        /////
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_config_new(
            NativeContext context,
            ulong input_data_size,
            ulong output_data_size,
            ulong function_definition_size,
            ulong host_exception_size,
            ulong guest_error_message_size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_config_get_guest_error_message_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern void mem_config_set_guest_error_message_size(
            NativeContext context,
            NativeHandle handle,
            ulong size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_config_get_host_function_definition_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern void mem_config_set_host_function_definition_size(
            NativeContext context,
            NativeHandle handle,
            ulong size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_config_get_host_exception_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_config_set_host_exception_size(
            NativeContext context,
            NativeHandle mem_config_handle,
            ulong size
        );


        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_config_get_input_data_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_config_set_input_data_size(
            NativeContext context,
            NativeHandle mem_config_handle,
            ulong size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_config_get_output_data_size(
            NativeContext context,
            NativeHandle mem_config_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_config_set_output_data_size(
            NativeContext context,
            NativeHandle mem_config_handle,
            ulong size
        );

#pragma warning restore CA5393
#pragma warning restore CA1401
#pragma warning restore CA1707



    }
}
