using System;
using System.Runtime.InteropServices;

namespace Hyperlight.Wrapper
{
    public class HandleWithGCHandle : IDisposable
    {
        private bool disposedValue;
        private readonly GCHandle gcHandle;
        private readonly Handle hdlWrapper;

        /// <summary>
        /// A convenience property to access the NativeHandle
        /// wrapped by the enclosed Handle
        /// the enclosed 
        /// </summary>
        public Context ctx => hdlWrapper.ctx;

        /// <summary>
        /// A convenience property to access the NativeHandle
        /// wrapped by the enclosed Handle
        /// </summary>
        public NativeHandle handle => hdlWrapper.handle;

        public HandleWithGCHandle(
            GCHandle gcHandle,
            Handle hdl
        )
        {
            this.gcHandle = gcHandle;
            this.hdlWrapper = hdl;
        }

        public void Dispose()
        {
            // Do not change this code. Put cleanup code in 
            // 'Dispose(bool disposing)' method
            this.Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                // the handle references the function pointer, so
                // dispose the handle before freeing the 
                // function pointer

                if (disposing)
                {
                    // dispose managed state (managed objects)

                    // dispose the handle
                    this.hdlWrapper.Dispose();


                    // now that the handle is freed, we can free the 
                    // GCHandle's associated memory.
                    //
                    // the GCHandle has to be handled with some care here.
                    // first, we need to call Free to release memory for the 
                    // underlying object (which is not managed by GC).
                    //
                    // then, the Target property of the GCHandle no longer
                    // references valid memory (because we've called Free)
                    // so we need to clean it up that, which we do by calling 
                    // Dispose
                    if (this.gcHandle.IsAllocated)
                    {
                        this.gcHandle.Free();
                    }
                    var gcHdlTarget = this.gcHandle.Target as IDisposable;
                    if (null != gcHdlTarget)
                    {
                        gcHdlTarget.Dispose();
                    }
                }

                disposedValue = true;
            }
        }

        ~HandleWithGCHandle()
        {
            // Do not change this code.
            // Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: false);
        }
    }

}
