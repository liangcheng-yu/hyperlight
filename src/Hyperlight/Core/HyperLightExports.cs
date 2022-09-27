using System;
using System.Runtime.InteropServices;
using HyperlightDependencies;
using Hyperlight.Native;

namespace Hyperlight.Core
{
    [ExposeToGuest(true)]
    public sealed class HyperLightExports
    {
        /// <summary>
        /// This is required by dlmalloc in HyperlightGuest
        /// </summary>
#pragma warning disable CA1024 // Use properties where appropriate - Intentional as properties cannot be exposed to guest
        public static long GetTickCount() => Environment.TickCount64;
#pragma warning restore CA1024 // Use properties where appropriate

        /// <summary>
        /// This is required by os_thread_get_stack_boundary() in WAMR (which is needed to handle AOT compiled WASM)
        /// </summary>
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

        /// <summary>
        /// This is required by os_getpagesize() in WAMR (which is needed to handle AOT compiled WASM)
        /// It is also used by dlmalloc 
        /// </summary>
#pragma warning disable CA1024 // Use properties where appropriate - Intentional as properties cannot be exposed to guest
        public static uint GetOSPageSize() => OS.GetPageSize();

        /// <summary>
        /// This is required by os_time_get_boot_microsecond() in WAMR 
        /// This is used for profiling and logging.
        /// Environment.TickCount has millisecond resolution and there is
        /// no simple way of getting data in correct resolution with dotnet
        /// So using DateTime.UtcNow as its unlikely clock adjustment is going to be an issue for short lived VMs
        /// </summary>
        /// <returns>microseconds since Unix epoch</returns>
        public static long GetTimeSinceBootMicrosecond()
        {
            var epoch = new DateTimeOffset(1970, 1, 1, 0, 0, 0, TimeSpan.Zero);
            var utcnow = DateTimeOffset.UtcNow;
            var unixTime = utcnow - epoch;
            return unixTime.Ticks / 10;
        }
#pragma warning restore CA1024 // Use properties where appropriate  
    }
}
