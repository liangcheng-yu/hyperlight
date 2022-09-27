using System;
using System.Runtime.InteropServices;
using HyperlightDependencies;
using Hyperlight.Native;

namespace Hyperlight.Core
{
    [ExposeToGuest(true)]
    public sealed class HyperLightExports
    {
#pragma warning disable CA1024 // Use properties where appropriate - Intentional as properties cannot be exposed to guest
        public static long GetTickCount() => Environment.TickCount64;
#pragma warning restore CA1024 // Use properties where appropriate

        public static IntPtr GetStackBoundary()
        {
            //TODO: This should be implemented for Linux if we re-enable in process support on Linux.

            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                // This implementation mirrors the implementation in WAMR at 
                // https://github.com/bytecodealliance/wasm-micro-runtime/blob/main/core/shared/platform/windows/win_thread.c#L665

                OS.GetCurrentThreadStackLimits(out var low, out var _);
                var page_size = OS.GetPageSize();

                // 4 pages are set unaccessible by system, we reserved
                // one more page at least for safety 

                return new IntPtr(low.ToInt64() + (long)page_size * 5);
            }

            throw new NotSupportedException();
        }
    }
}
