using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;
namespace Hyperlight.Wrapper
{
    public abstract class SandboxMemoryManager : IDisposable
    {
        private bool disposedValue;
        protected readonly Context ctxWrapper;
        private readonly Handle hdlMemManagerWrapper;
        internal abstract GuestMemory GetGuestMemory();
        internal abstract SandboxMemoryLayout GetSandboxMemoryLayout();
        internal abstract ulong GetSize();
        internal abstract IntPtr GetSourceAddress();

        protected SandboxMemoryManager(
            Context ctxWrapper,
            SandboxMemoryConfiguration memCfg,
            bool runFromProcessMemory
        )
        {
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
            var guestMemWrapper = this.GetGuestMemory();
            var sandboxMemoryLayout = this.GetSandboxMemoryLayout();
            HyperlightException.ThrowIfNull(
                cookie,
                nameof(cookie),
                GetType().Name
            );
            HyperlightException.ThrowIfNull(
                sandboxMemoryLayout,
                nameof(sandboxMemoryLayout),
                GetType().Name
            );
            HyperlightException.ThrowIfNull(
                guestMemWrapper,
                nameof(guestMemWrapper),
                GetType().Name
            );
            using var cookieByteArray = new ByteArray(
                this.ctxWrapper,
                cookie
            );
            var rawHdl = mem_mgr_set_stack_guard(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                sandboxMemoryLayout.rawHandle,
                guestMemWrapper.handleWrapper.handle,
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
            var guestMemWrapper = this.GetGuestMemory();
            var sandboxMemoryLayout = this.GetSandboxMemoryLayout();
            HyperlightException.ThrowIfNull(
                cookie,
                nameof(cookie),
                GetType().Name
            );
            HyperlightException.ThrowIfNull(
                sandboxMemoryLayout,
                nameof(sandboxMemoryLayout),
                GetType().Name
            );
            HyperlightException.ThrowIfNull(
                guestMemWrapper,
                nameof(guestMemWrapper),
                GetType().Name
            );
            using var cookieByteArray = new ByteArray(
                this.ctxWrapper,
                cookie
            );
            var rawHdl = mem_mgr_check_stack_guard(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                sandboxMemoryLayout.rawHandle,
                guestMemWrapper.handleWrapper.handle,
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
            var guestMemWrapper = this.GetGuestMemory();
            var sandboxMemoryLayout = this.GetSandboxMemoryLayout();
            var size = this.GetSize();
            HyperlightException.ThrowIfNull(
                guestMemWrapper,
                nameof(guestMemWrapper),
                GetType().Name
            );
            var rawHdl = mem_mgr_set_up_hypervisor_partition(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                guestMemWrapper.handleWrapper.handle,
                size
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
            var sandboxMemoryLayout = this.GetSandboxMemoryLayout();
            var sourceAddress = this.GetSourceAddress();
            HyperlightException.ThrowIfNull(
                sandboxMemoryLayout,
                nameof(sandboxMemoryLayout),
                GetType().Name
            );
            var rawHdl = mem_mgr_get_peb_address(
                this.ctxWrapper.ctx,
                this.hdlMemManagerWrapper.handle,
                sandboxMemoryLayout.rawHandle,
                (ulong)sourceAddress.ToInt64()
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException("mem_mgr_get_peb_address did not return a uint64");
            }
            return hdl.GetUInt64();
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
            NativeHandle guestMemHdl,
            NativeHandle cookieHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_check_stack_guard(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle layoutHdl,
            NativeHandle guestMemHdl,
            NativeHandle cookieHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_set_up_hypervisor_partition(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle guestMemHdl,
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

#pragma warning restore CA1707 // Remove the underscores from member name
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory


    }
}
