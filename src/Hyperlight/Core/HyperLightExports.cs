using System;
using System.Runtime.InteropServices;
using HyperlightDependencies;
using Hyperlight.Native;
using System.Reflection;
using System.Diagnostics;

namespace Hyperlight.Core
{
    [ExposeToGuest(true)]
    public sealed class HyperLightExports
    {
        private static readonly DateTimeOffset epoch = new DateTimeOffset(1970, 1, 1, 0, 0, 0, TimeSpan.Zero);
        /// <summary>
        /// This is required by dlmalloc in HyperlightGuest
        /// </summary>
#pragma warning disable CA1024 // Use properties where appropriate - Intentional as properties cannot be exposed to guest

        public static long GetTickCount() => Stopwatch.GetTimestamp();

        /// <summary>
        /// This is required by os_thread_get_stack_boundary() in WAMR (which is needed to handle AOT compiled WASM)
        /// </summary>
        public static IntPtr GetStackBoundary()
        {
            //TODO: This should be implemented for Linux if we re-enable in process support on Linux.
            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                HyperlightException.LogAndThrowException<NotSupportedException>($"Hyperlight only supports in process execution on Windows.", Sandbox.CorrelationId.Value!, MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            }

            // This implementation mirrors the implementation in WAMR at 
            // https://github.com/bytecodealliance/wasm-micro-runtime/blob/main/core/shared/platform/windows/win_thread.c#L665

            OS.GetCurrentThreadStackLimits(out var low, out var _);
            var page_size = OS.GetPageSize();

            // 4 pages are set unaccessible by system, we reserved
            // one more page at least for safety 

            return new IntPtr(low.ToInt64() + (long)page_size * 5);

        }

        /// <summary>
        /// This is required by os_getpagesize() in WAMR (which is needed to handle AOT compiled WASM)
        /// It is also used by dlmalloc 
        /// </summary>
        public static uint GetOSPageSize() => OS.GetPageSize();

        /// <summary>
        /// This is required by os_time_get_boot_microsecond() in WAMR 
        /// This is used for profiling and logging.
        /// Environment.TickCount has millisecond resolution and there is
        /// no simple way of getting data in correct resolution with dotnet
        /// So using DateTime.UtcNow as its unlikely clock adjustment is going to be an issue for short lived VMs
        /// </summary>
        /// <returns>microseconds since Unix epoch</returns>
        public static long GetTimeSinceBootMicrosecond() => (DateTimeOffset.UtcNow - epoch).Ticks / 10;
#pragma warning restore CA1024 // Use properties where appropriate  
    }
}
