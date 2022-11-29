using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    internal class SandboxMemoryLayout : IDisposable
    {
        static readonly ulong pageTableSize = mem_layout_get_page_table_size();
        public static ulong PML4Offset = mem_layout_get_pml4_offset();
        public static ulong PDOffset = mem_layout_get_pd_offset();
        public static ulong PDPTOffset = mem_layout_get_pdpt_offset();
        public static ulong CodeOffSet = pageTableSize;
        public static ulong BaseAddress = mem_layout_get_base_address();
        public static ulong PML4GuestAddress = BaseAddress;
        public static ulong PDPTGuestAddress = BaseAddress + PDPTOffset;
        public static ulong PDGuestAddress = BaseAddress + PDOffset;
        public static ulong GuestCodeAddress = BaseAddress + CodeOffSet;

        private long callAddrFn(
            Func<NativeContext, NativeHandle, long, long> fn,
            long base_addr
        )
        {
            return fn(this.ctxWrapper.ctx, this.handleWrapper.handle, base_addr);
        }

        private long addrToOffset(
            Func<NativeContext, NativeHandle, long, long> fn
        )
        {
            return this.callAddrFn(fn, 0);
        }
#pragma warning disable IDE0051 // Member is Unused - this is only used by tests
        // TODO: Remove this when port to rust is done and the test is no longer needed.
        long stackSize => (long)mem_layout_get_stack_size(
             this.ctxWrapper.ctx,
             this.handleWrapper.handle
         );
#pragma warning restore IDE0051 // Member is Unused

        public long hostExceptionOffset => this.addrToOffset(
            mem_layout_get_host_exception_address
        );

        public long outbPointerOffset => this.addrToOffset(
            mem_layout_get_code_and_outb_pointer_address
        ) + sizeof(ulong);

        public long outputDataOffset => this.addrToOffset(
            mem_layout_get_output_data_address
        );

        public long guestErrorOffset => this.addrToOffset(
            mem_layout_get_guest_error_address
        );

        public long inputDataBufferOffset => this.addrToOffset(
            mem_layout_get_input_data_buffer_address
        );

        public long topOfStackOffset => this.addrToOffset(
            mem_layout_get_top_of_stack_address
        );

        public long outputDataBufferOffset => this.addrToOffset(
            mem_layout_get_output_data_buffer_address
        );

        public long codePointerAddressOffset => this.addrToOffset(
                mem_layout_get_code_pointer_address
        );

        public long guestErrorMessageBufferOffset => this.addrToOffset(
                mem_layout_get_guest_error_message_buffer_address
        );

        public long dispatchFunctionPointerOffSet => this.addrToOffset(
            mem_layout_get_dispatch_function_pointer_address
        );

        public long PEBAddress => mem_layout_get_peb_address(
            this.ctxWrapper.ctx,
            this.handleWrapper.handle
        );

        public long guestErrorAddressOffset => this.addrToOffset(
            mem_layout_get_guest_error_address
        );

#pragma warning disable IDE0051 // Member is Unused - this is only used by tests
        // TODO: Remove this when port to rust is done and the test is no longer needed.
        private IntPtr GetGuestErrorMessageSizeAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_guest_error_message_size_address,
                address.ToInt64()
            ));
        }
#pragma warning restore IDE0051 // Member is Unused

        public IntPtr GetGuestErrorMessagePointerAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_guest_error_message_pointer_address,
                address.ToInt64()
            ));
        }

        public IntPtr GetFunctionDefinitionAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_host_function_definitions_address,
                address.ToInt64()
            ));
        }

#pragma warning disable IDE0051 // Member is Unused - these are only used by tests
        // TODO: Remove this when port to rust is done and the test is no longer needed.

        IntPtr GetFunctionDefinitionSizeAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_host_function_definitions_size_address,
                address.ToInt64()
            ));
        }

        IntPtr GetHostExceptionSizeAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_host_exception_size_address,
                address.ToInt64()
            ));
        }
#pragma warning restore IDE0051 // Member is Unused

        public IntPtr GetHostExceptionAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_host_exception_address,
                address.ToInt64()
            ));
        }

#pragma warning disable IDE0051 // Member is Unused - this is only used by tests
        // TODO: Remove this when port to rust is done and the test is no longer needed.

        IntPtr GetOutputDataSizeAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_output_data_size_address,
                address.ToInt64()
            ));
        }
#pragma warning restore IDE0051 // Member is Unused

        public IntPtr GetOutputDataAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_output_data_address,
                address.ToInt64()
            ));
        }

#pragma warning disable IDE0051 // Member is Unused - this is only used by tests
        // TODO: Remove this when port to rust is done and the test is no longer needed.
        IntPtr GetInputDataSizeAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_input_data_size_address,
                address.ToInt64()
            ));
        }
#pragma warning restore IDE0051 // Member is Unused

        public long GetInProcessPEBAddress(IntPtr address)
        {
            return this.callAddrFn(
                mem_layout_get_in_process_peb_address,
                address.ToInt64()
            );
        }

#pragma warning disable IDE0051 // Member is Unused - this is only used by tests
        // TODO: Remove this when port to rust is done and the test is no longer needed.
        IntPtr GetHeapSizeAddress(IntPtr address)
        {
            return new IntPtr(this.callAddrFn(
                mem_layout_get_heap_size_address,
                address.ToInt64()
            ));
        }
#pragma warning restore IDE0051 // Member is Unused

        public static IntPtr GetHostCodeAddress(IntPtr address)
        {
            return new IntPtr(
                mem_layout_get_host_code_address(address.ToInt64())
            );
        }

        readonly Wrapper.Context ctxWrapper;
        readonly Wrapper.Handle handleWrapper;

        private bool disposed;
        internal SandboxMemoryLayout(
            SandboxMemoryConfiguration sandboxMemoryConfiguration,
            ulong codeSize,
            ulong stackSize,
            ulong heapSize
        )
        {
            HyperlightException.ThrowIfNull(Sandbox.Context.Value, Sandbox.CorrelationId.Value!, GetType().Name);
            this.ctxWrapper = Sandbox.Context.Value;
            var rawHandle = mem_layout_new(
                ctxWrapper.ctx,
                sandboxMemoryConfiguration.Handle.handle,
                codeSize,
                stackSize,
                heapSize
            );
            this.handleWrapper = new Wrapper.Handle(ctxWrapper, rawHandle);
            NativeHandleWrapperErrorExtensions.ThrowIfError(this.handleWrapper);
            var memSize = mem_layout_get_memory_size(
                ctxWrapper.ctx,
                rawHandle
            );
            if (memSize == 0)
            {
                HyperlightException.LogAndThrowException("Memory requested exceeds maximum size allowed", Sandbox.CorrelationId.Value!, GetType().Name);
            }
        }

        internal ulong GetMemorySize()
        {
            return mem_layout_get_memory_size(
                this.ctxWrapper.ctx,
                this.handleWrapper.handle
            );

        }

        internal void WriteMemoryLayout(
            Wrapper.GuestMemory guestMemoryWrapper,
            IntPtr guestAddress,
            ulong size
        )
        {
            var hdlRes = mem_layout_write_memory_layout(
                this.ctxWrapper.ctx,
                this.handleWrapper.handle,
                guestMemoryWrapper.handleWrapper.handle,
                (ulong)guestAddress.ToInt64(),
                size
            );
            using var hdlWrapper = new Handle(this.ctxWrapper, hdlRes);
            hdlWrapper.ThrowIfError();
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
                    this.handleWrapper.Dispose();
                }
                this.disposed = true;
            }
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_layout_new(
            NativeContext ctx,
            NativeHandle mem_config_handle,
            ulong code_size,
            ulong stack_size,
            ulong heap_size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_stack_size(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_heap_size(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_host_functions_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long base_addr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_host_exception_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long base_addr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_code_and_outb_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long base_addr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_output_data_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long base_addr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_input_data_buffer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long base_addr
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_output_data_buffer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long base_addr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_peb_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_guest_error_address
        (
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long base_addr
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_guest_error_message_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long base_addr
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_guest_error_message_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_host_function_definitions_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_host_function_definitions_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_host_exception_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_output_data_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_input_data_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_code_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_guest_error_message_buffer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_dispatch_function_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_in_process_peb_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_heap_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_top_of_stack_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_top_of_stack_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_page_table_size();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_base_address();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_pml4_offset();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_pd_offset();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_pdpt_offset();

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern long mem_layout_get_host_code_address(
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_layout_write_memory_layout(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            NativeHandle guest_memory_handle,
            ulong guestOffset,
            ulong size
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_memory_size(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

#pragma warning restore CA5393
#pragma warning restore CA1707
    }
}
