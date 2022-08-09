using System;
namespace Hyperlight.Core
{
    public class HandleWrapper : IDisposable
    {
        public ContextWrapper ctx { get; private set; }
        public NativeHandle handle { get; private set; }
        private bool disposed;

        public HandleWrapper(ContextWrapper ctx, NativeHandle hdl)
        {
            this.ctx = ctx;
            this.handle = hdl;
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
                    NativeWrapper.handle_free(this.ctx.ctx, this.handle);
                }
                disposed = true;
            }
        }
    }
}
