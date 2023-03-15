using System;
using Hyperlight.Native;

namespace Hyperlight.Wrapper
{
    internal sealed class LoadLibrary : IDisposable
    {
        private readonly IntPtr loadAddrVal;
        internal IntPtr LoadAddr => loadAddrVal;
        private readonly bool disposed;

        internal LoadLibrary(string path)
        {
            this.loadAddrVal = OS.LoadLibrary(lpFileName: path);
            this.disposed = false;
        }

        public void Dispose()
        {
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

        private void Dispose(bool disposing)
        {
            if (!this.disposed)
            {
                if (disposing)
                {
                    // dispose managed resources here
                }
                OS.FreeLibrary(this.loadAddrVal);
            }
        }

        ~LoadLibrary()
        {
            Dispose(false);
        }
    }
}
