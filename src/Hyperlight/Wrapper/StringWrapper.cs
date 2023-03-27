using System;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    public class StringWrapper : IDisposable
    {
        private readonly Context ctxWrapper;
        private readonly Handle hdl;
        public Handle HandleWrapper { get { return hdl; } }
        private bool disposed;

        private StringWrapper(
            Context ctxWrapper,
            NativeHandle rawHdl
        )
        {
            HyperlightException.ThrowIfNull(
                ctxWrapper,
                nameof(ctxWrapper),
                GetType().Name
            );
            this.hdl = new Handle(ctxWrapper, rawHdl, true);
            this.ctxWrapper = ctxWrapper;
        }
        public static StringWrapper FromString(
            Context ctxWrapper,
            string str
        )
        {
            HyperlightException.ThrowIfNull(
                ctxWrapper,
                nameof(ctxWrapper),
                MethodBase.GetCurrentMethod()!.DeclaringType!.Name
            );
            HyperlightException.ThrowIfNull(
                str,
                nameof(str),
                MethodBase.GetCurrentMethod()!.DeclaringType!.Name
            );
            var rawHdl = string_new(ctxWrapper.ctx, str);
            return new StringWrapper(
                ctxWrapper,
                rawHdl
            );
        }

        public override string ToString()
        {
            return $"StringWrapper: {this.HandleWrapper.GetString()}";
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
                    this.hdl.Dispose();
                }
                this.disposed = true;
            }
        }


#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning disable CA2101 // Specify marshaling for P/Invoke string arguments

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern unsafe NativeHandle string_new(
            NativeContext ctx,
            [MarshalAs(UnmanagedType.LPStr)] string str
        );

#pragma warning restore CA2101 // Specify marshaling for P/Invoke string arguments
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707
    }
}
