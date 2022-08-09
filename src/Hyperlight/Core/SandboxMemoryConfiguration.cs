using System;
namespace Hyperlight.Core
{
    public class SandboxMemoryConfiguration : IDisposable
    {
        const ulong DefaultInputSize = 0x4000;
        const ulong DefaultOutputSize = 0x4000;
        const ulong DefaultHostFunctionDefinitionSize = 0x1000;
        const ulong DefaultHostExceptionSize = 0x1000;
        const ulong DefaultGuestErrorMessageSize = 0x100;

        private ContextWrapper ctxWrapper;
        private HandleWrapper hdlWrapper;
        private bool disposed;

        public SandboxMemoryConfiguration(
            ContextWrapper ctx,
            ulong inputDataSize = DefaultInputSize,
            ulong outputDataSize = DefaultOutputSize,
            ulong functionDefinitionSize = DefaultHostFunctionDefinitionSize,
            ulong hostExceptionSize = DefaultHostExceptionSize,
            ulong guestErrorMessageSize = DefaultGuestErrorMessageSize
        )
        {
            if (ctx == null)
            {
                throw new ArgumentNullException(nameof(ctx));
            }
            var rawHandle = NativeWrapper.mem_config_new(
                ctx.ctx,
                inputDataSize,
                outputDataSize,
                functionDefinitionSize,
                hostExceptionSize,
                guestErrorMessageSize
            );
            this.ctxWrapper = ctx;
            this.hdlWrapper = new HandleWrapper(ctx, rawHandle);
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
            }
        }

        /// <summary>
        /// GuestErrorMessageSize defines the maximum size of the guest error message field.
        /// </summary>
        public ulong GuestErrorMessageSize
        {
            get
            {
                return NativeWrapper.mem_config_get_guest_error_message_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                NativeWrapper.mem_config_set_guest_error_message_size(
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
                return NativeWrapper.mem_config_get_host_function_definition_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                NativeWrapper.mem_config_set_host_function_definition_size(
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
                return NativeWrapper.mem_config_get_host_exception_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                NativeWrapper.mem_config_set_host_exception_size(
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
                return NativeWrapper.mem_config_get_input_data_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                NativeWrapper.mem_config_set_input_data_size(
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
                return NativeWrapper.mem_config_get_output_data_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle
                );
            }
            set
            {
                NativeWrapper.mem_config_set_output_data_size(
                    this.ctxWrapper.ctx,
                    this.hdlWrapper.handle,
                    value
                );
            }
        }



    }
}
