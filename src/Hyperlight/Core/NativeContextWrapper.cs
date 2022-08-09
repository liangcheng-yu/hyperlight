using System;
namespace Hyperlight.Core
{
    public class ContextWrapper : IDisposable
    {
        public ContextWrapper() : this(NativeWrapper.context_new())
        {
        }

        public NativeContext ctx { get; private set; }
        private bool disposed;
        public ContextWrapper(NativeContext ctx)
        {
            this.ctx = ctx;
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
                    NativeWrapper.context_free(this.ctx);
                }
                this.disposed = true;
            }
        }
    }
}
