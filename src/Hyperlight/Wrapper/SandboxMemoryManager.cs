using System;
using System.Runtime.InteropServices;
using System.Text;
using Hyperlight.Core;
using Hyperlight.Generated;
using Newtonsoft.Json;

namespace Hyperlight.Wrapper
{
    public abstract class SandboxMemoryManager : IDisposable
    {
        public ulong EntryPoint { get; internal set; }
        private bool disposedValue;
        private readonly Context ctxWrapper;
        private readonly Handle hdlMemManagerWrapper;
        protected Context ContextWrapper => ctxWrapper;
        internal SharedMemory? sharedMemoryWrapper;
        internal SandboxMemoryLayout? sandboxMemoryLayout;

        internal readonly bool runFromProcessMemory;
        internal readonly SandboxMemoryConfiguration sandboxMemoryConfiguration;
        internal ulong size;
        internal IntPtr sourceAddress;
        internal ulong Size { get { return size; } }
        internal IntPtr SourceAddress { get { return sourceAddress; } }
        internal SharedMemory SharedMemoryWrapper
        {
            get
            {
                var sharedMemWrapper = this.sharedMemoryWrapper;
                HyperlightException.ThrowIfNull(
                    sharedMemWrapper,
                    nameof(sharedMemWrapper),
                    GetType().Name
                );
                return sharedMemWrapper;
            }
        }

        internal SandboxMemoryLayout SandboxMemoryLayoutWrapper
        {
            get
            {
                var sandboxMemoryLayout = this.sandboxMemoryLayout;
                HyperlightException.ThrowIfNull(
                    sandboxMemoryLayout,
                    nameof(sandboxMemoryLayout),
                    GetType().Name
                );
                return sandboxMemoryLayout;
            }
        }

        protected SandboxMemoryManager(
            Context ctxWrapper,
            SandboxMemoryConfiguration memCfg,
            bool runFromProcessMemory
        )
        {
            this.sandboxMemoryConfiguration = memCfg;
            this.runFromProcessMemory = runFromProcessMemory;
            HyperlightException.ThrowIfNull(
                ctxWrapper,
                nameof(ctxWrapper),
                GetType().Name
            );
            this.ctxWrapper = ctxWrapper;
            var rawHdl = mem_mgr_new(
                    ctxWrapper.ctx,
                    memCfg,
                    runFromProcessMemory
                );
            this.hdlMemManagerWrapper = new Handle(ctxWrapper, rawHdl, true);
        }

        internal void SetStackGuard(byte[] cookie)
        {
            HyperlightException.ThrowIfNull(
                cookie,
                nameof(cookie),
                GetType().Name
            );
            using var cookieByteArray = new ByteArray(
                this.ctxWrapper,
                cookie
            );
            var rawHdl = mem_mgr_set_stack_guard(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,

                this.SandboxMemoryLayoutWrapper.rawHandle,
                this.SharedMemoryWrapper.handleWrapper.handle,
                cookieByteArray.handleWrapper.handle
            );
            using var hdl = new Handle(
                this.ctxWrapper,
                rawHdl,
                true
            );
        }

        internal bool CheckStackGuard(byte[]? cookie)
        {
            HyperlightException.ThrowIfNull(
                cookie,
                nameof(cookie),
                GetType().Name
            );
            using var cookieByteArray = new ByteArray(
                this.ctxWrapper,
                cookie
            );
            var rawHdl = mem_mgr_check_stack_guard(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SandboxMemoryLayoutWrapper.rawHandle,
                this.SharedMemoryWrapper.handleWrapper.handle,
                cookieByteArray.handleWrapper.handle
            );
            using var hdl = new Handle(
                this.ctxWrapper,
                rawHdl,
                true
            );
            if (!hdl.IsBoolean())
            {
                throw new HyperlightException("call to rust mem_mgr_check_stack_guard` did not return an error nor a boolean");
            }
            return hdl.GetBoolean();
        }

        internal ulong SetUpHyperVisorPartition()
        {
            var rawHdl = mem_mgr_set_up_hypervisor_partition(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SharedMemoryWrapper.handleWrapper.handle,
                this.Size
            );
            using var hdl = new Handle(
                this.ctxWrapper,
                rawHdl,
                true
            );
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException("mem_mgr_set_up_hypervisor_partition did not return a UInt64");
            }
            return hdl.GetUInt64();
        }

        internal ulong GetPebAddress()
        {
            var rawHdl = mem_mgr_get_peb_address(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SandboxMemoryLayoutWrapper.rawHandle,
                (ulong)this.SourceAddress.ToInt64()
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException("mem_mgr_get_peb_address did not return a uint64");
            }
            return hdl.GetUInt64();
        }

        internal void SnapshotState()
        {
            var rawHdl = mem_mgr_snapshot_state(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SharedMemoryWrapper.handleWrapper.handle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
        }

        internal void RestoreState()
        {
            var rawHdl = mem_mgr_restore_state(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
        }

        internal int GetReturnValue()
        {
            var rawHdl = mem_mgr_get_return_value(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SharedMemoryWrapper.handleWrapper.handle,
                this.SandboxMemoryLayoutWrapper.rawHandle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsInt32())
            {
                throw new HyperlightException(
                    "handle returned from mem_mgr_get_return_value was not an int32"
                );
            }
            return hdl.GetInt32();
        }

        internal void SetOutBAddress(long pOutB)
        {
            var rawHdl = mem_mgr_set_outb_address(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SharedMemoryWrapper.handleWrapper.handle,
                this.SandboxMemoryLayoutWrapper.rawHandle,
                (ulong)pOutB
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
        }

        internal string? GetHostCallMethodName()
        {
            var rawHdl = mem_mgr_get_host_call_method_name(
                this.ContextWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SharedMemoryWrapper.handleWrapper.handle,
                this.SandboxMemoryLayoutWrapper.rawHandle
            );
            using var hdl = new Handle(this.ContextWrapper, rawHdl, true);
            if (!hdl.IsString())
            {
                throw new HyperlightException("mem_mgr_get_host_call_method_name did not return a Handle referencing a string");
            }
            return hdl.GetString();
        }

        internal long GetAddressOffset()
        {
            var rawHdl = mem_mgr_get_address_offset(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                (ulong)this.SourceAddress.ToInt64()
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException("mem_mgr_get_address_offset did not return a uint64");
            }
            return (long)hdl.GetUInt64();
        }
        internal ulong GetPointerToDispatchFunction()
        {
            var rawHdl = mem_mgr_get_pointer_to_dispatch_function(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SharedMemoryWrapper.handleWrapper.handle,
                this.SandboxMemoryLayoutWrapper.rawHandle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException(
                    "mem_mgr_get_pointer_to_dispatch_function did not return a uint64"
                );
            }
            return hdl.GetUInt64();
        }

        internal IntPtr GetHostAddressFromPointer(long address)
        {
            var rawHdl = mem_mgr_get_host_address_from_pointer(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SharedMemoryWrapper.handleWrapper.handle,
                (ulong)address
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException("mem_mgr_get_host_address_from_pointer did not return a uint64");
            }
            return (IntPtr)hdl.GetUInt64();
        }

        internal IntPtr GetGuestAddressFromPointer(IntPtr address)
        {
            var rawHdl = mem_mgr_get_guest_address_from_pointer(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SharedMemoryWrapper.handleWrapper.handle,
                (ulong)address.ToInt64()
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException("mem_mgr_get_guest_address_from_pointer did not return a uint64");
            }
            return (IntPtr)hdl.GetUInt64();
        }

        internal string? ReadStringOutput()
        {
            var rawHdl = mem_mgr_read_string_output(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SandboxMemoryLayoutWrapper.rawHandle,
                this.SharedMemoryWrapper.handleWrapper.handle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsString())
            {
                throw new HyperlightException("mem_mgr_read_string_output did not return a string");
            }
            return hdl.GetString();
        }

        internal HyperlightException? GetHostException()
        {
            HyperlightException? hyperlightException = null;
            var rawHdl = mem_mgr_has_host_exception(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                this.SandboxMemoryLayoutWrapper.rawHandle,
                this.SharedMemoryWrapper.handleWrapper.handle
            );
            using var hdl1 = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl1.IsBoolean())
            {
                throw new HyperlightException("mem_mgr_has_host_exception did not return a boolean");
            }
            var hasException = hdl1.GetBoolean();
            if (hasException)
            {
                rawHdl = mem_mgr_get_host_exception_length(
                    this.ctxWrapper.ctx,
                    this.hdlMemManagerWrapper.handle,
                    this.SandboxMemoryLayoutWrapper.rawHandle,
                    this.SharedMemoryWrapper.handleWrapper.handle
                );
                using var hdl2 = new Handle(this.ctxWrapper, rawHdl, true);
                if (!hdl2.IsInt32())
                {
                    throw new HyperlightException("mem_mgr_get_host_exception_length did not return an int32");
                }
                var size = hdl2.GetInt32();
                if (size == 0)
                {
                    throw new HyperlightException("mem_mgr_get_host_exception_length returned 0");
                }
                var data = new byte[size];
                unsafe
                {
                    fixed (byte* exceptionDataPtr = data)
                    {
                        using var resultHdlWrapper = new Handle(
                            this.ctxWrapper,
                            mem_mgr_get_host_exception_data(
                                this.ctxWrapper.ctx,
                                this.hdlMemManagerWrapper.handle,
                                this.SandboxMemoryLayoutWrapper.rawHandle,
                                this.SharedMemoryWrapper.handleWrapper.handle,
                                (IntPtr)exceptionDataPtr,
                                size
                            ),
                            true
                        );
                    }
                }
                var exceptionAsJson = Encoding.UTF8.GetString(data);
                // TODO: Switch to System.Text.Json - requires custom serialisation as default throws an exception when serialising if an inner exception is present
                // as it contains a Type: System.NotSupportedException: Serialization and deserialization of 'System.Type' instances are not supported and should be avoided since they can lead to security issues.
                // https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-converters-how-to?pivots=dotnet-6-0
#pragma warning disable CA2326 // Do not use TypeNameHandling values other than None - this will be fixed by the above TODO
#pragma warning disable CA2327 // Do not use SerializationBinder classes - this will be fixed by the above TODO 
                hyperlightException = JsonConvert.DeserializeObject<HyperlightException>(exceptionAsJson, new JsonSerializerSettings
                {
                    TypeNameHandling = TypeNameHandling.Auto
                });
#pragma warning restore CA2326 // Do not use TypeNameHandling values other than None
#pragma warning restore CA2327 // Do not use SerializationBinder classes
            }
            return hyperlightException;
        }

        internal void WriteOutbException(Exception ex, ushort port)
        {
            var data = Encoding.UTF8.GetBytes($"Port:{port}, Message:{ex.Message}\0");
            using var errorMessage = new ByteArray(
                this.ctxWrapper,
                data
            );

            var hyperLightException = ex.GetType() == typeof(HyperlightException) ? ex as HyperlightException : new HyperlightException($"OutB Error {ex.Message}", ex);

            // TODO: Switch to System.Text.Json - requires custom serialisation as default throws an exception when serialising if an inner exception is present
            // as it contains a Type: System.NotSupportedException: Serialization and deserialization of 'System.Type' instances are not supported and should be avoided since they can lead to security issues.
            // https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-converters-how-to?pivots=dotnet-6-0
#pragma warning disable CA2326 // Do not use TypeNameHandling values other than None - this will be fixed by the above TODO
            var exceptionAsJson = JsonConvert.SerializeObject(hyperLightException, new JsonSerializerSettings
            {
                TypeNameHandling = TypeNameHandling.Auto
            });

#pragma warning restore CA2326 // Do not use TypeNameHandling values other than None
            data = Encoding.UTF8.GetBytes(exceptionAsJson);
            using var exceptionData = new ByteArray(
                this.ctxWrapper,
                data
            );

            using var resultHdlWrapper = new Handle(
                this.ctxWrapper,
                mem_mgr_write_outb_exception(
                    this.ctxWrapper.ctx,
                    this.hdlMemManagerWrapper.handle,
                    this.SandboxMemoryLayoutWrapper.rawHandle,
                    this.SharedMemoryWrapper.handleWrapper.handle,
                    errorMessage.handleWrapper.handle,
                    exceptionData.handleWrapper.handle),
                true
            );

            HyperlightLogger.LogError($"Exception occurred in outb", GetType().Name, ex);
        }

        internal (ErrorCode ErrorCode, string? Message) GetGuestError()
        {

            using var resultHdlWrapper = new Handle(
                this.ctxWrapper,
                 mem_mgr_get_guest_error(
                    this.ctxWrapper.ctx,
                    this.hdlMemManagerWrapper.handle,
                    this.SandboxMemoryLayoutWrapper.rawHandle,
                    this.SharedMemoryWrapper.handleWrapper.handle),
                true
            );

            if (!resultHdlWrapper.IsGuestError())
            {
                throw new HyperlightException("mem_mgr_get_guest_error did not return a GuestError");
            }
            var guestError = resultHdlWrapper.GetGuestError();
            return (guestError.Code, guestError.Message);

        }

        /// <summary>
        /// A function for subclasses to implement if they want to implement
        /// any Dispose logic of their own.
        /// Subclasses should not re-implement any Dispose(...) functions, nor
        /// a finalizer. Instead, they should override this method. It will 
        /// be correctly called during disposal.
        /// </summary>
        protected virtual void DisposeHook(bool disposing) { }

        private void Dispose(bool disposing)
        {
            DisposeHook(disposing: disposing);
            // note that in both ~SandboxMemoryManager and Dispose(),
            // this method is called, but it's virtual, 
            // so the derived class's Dispose(disposing) method is
            // called. the derived method should, in its last line,
            // call base.Dispose(disposing) to call up to this!
            if (!disposedValue)
            {
                if (disposing)
                {
                    this.hdlMemManagerWrapper.Dispose();
                }
                disposedValue = true;
            }
        }


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

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_new(
            NativeContext ctx,
            SandboxMemoryConfiguration cfg,
            [MarshalAs(UnmanagedType.U1)] bool runFromProcessMemory
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_set_stack_guard(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle layoutHdl,
            NativeHandle sharedMemHdl,
            NativeHandle cookieHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_check_stack_guard(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle layoutHdl,
            NativeHandle sharedMemHdl,
            NativeHandle cookieHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_set_up_hypervisor_partition(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle sharedMemHdl,
            ulong mem_size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_peb_address(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle memLayoutHdl,
            ulong memStartAddr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_snapshot_state(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle sharedMemHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_restore_state(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_return_value(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle sharedMemHdl,
            NativeHandle layoutHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_set_outb_address(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle sharedMemHdl,
            NativeHandle layoutHdl,
            ulong addr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_host_call_method_name(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle guestMemHdl,
            NativeHandle layoutHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_address_offset(
            NativeContext ctx,
            NativeHandle mgrHdl,
            ulong sourceAddr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_host_address_from_pointer(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle sharedMemHdl,
            ulong addr
        );


        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_guest_address_from_pointer(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle sharedMemHdl,
            ulong addr
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_pointer_to_dispatch_function(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle sharedMemHdl,
            NativeHandle layoutHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_read_string_output(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle layoutHdl,
            NativeHandle sharedMemHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_has_host_exception(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle layoutHdl,
            NativeHandle sharedMemHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_host_exception_length(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle layoutHdl,
            NativeHandle sharedMemHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_host_exception_data(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle layoutHdl,
            NativeHandle sharedMemHdl,
            IntPtr exceptionDataPtr,
            int exceptionDataLen
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_write_outb_exception(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle layoutHdl,
            NativeHandle sharedMemHdl,
            NativeHandle guestErrorMsgHdl,
            NativeHandle hostExceptionDataHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_guest_error(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle layoutHdl,
            NativeHandle guestMemHdl
        );

#pragma warning restore CA1707 // Remove the underscores from member name
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory


    }
}
