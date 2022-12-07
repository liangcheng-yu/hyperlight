using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public class GuestMemorySnapshot : IDisposable
    {
        private readonly Context ctxWrapper;
        private readonly GuestMemory guestMemWrapper;
        private readonly Handle guestMemSnapshotWrapper;
        private bool disposed;
        public GuestMemorySnapshot(
            Context ctxWrapper,
            GuestMemory guestMemWrapper
        )
        {
            this.ctxWrapper = ctxWrapper;
            this.guestMemWrapper = guestMemWrapper;
            var rawHdl = guest_memory_snapshot_new(
                this.ctxWrapper.ctx,
                this.guestMemWrapper.handleWrapper.handle
            );
            this.guestMemSnapshotWrapper = new Handle(
                this.ctxWrapper,
                rawHdl,
                true
            );
        }

        public void ReplaceSnapshot()
        {
            var rawHdl = guest_memory_snapshot_replace(
                this.ctxWrapper.ctx,
                this.guestMemSnapshotWrapper.handle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
        }

        public void RestoreFromSnapshot()
        {
            var rawHdl = guest_memory_snapshot_restore(
                this.ctxWrapper.ctx,
                this.guestMemSnapshotWrapper.handle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
        }

        public void Dispose()
        {
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!this.disposed)
            {
                if (disposing)
                {
                    this.guestMemSnapshotWrapper.Dispose();
                }
                disposed = true;
            }
        }

        ~GuestMemorySnapshot()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: false);
        }
#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle guest_memory_snapshot_new(
            NativeContext ctx,
            NativeHandle guestMemHdl
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle guest_memory_snapshot_restore(
            NativeContext ctx,
            NativeHandle guestMemSnapshotHdl
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle guest_memory_snapshot_replace(
            NativeContext ctx,
            NativeHandle guestMemSnapshotHdl
        );
#pragma warning restore CA1707 // Remove the underscores from member name
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
    }
}
