using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using HyperlightDependencies;

namespace Hyperlight.Core
{
    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
#pragma warning disable CA1815 // Override equals and operator equals on value types
    public readonly struct Duration
    {
        // Do not change the ordering of these struct fields
        // without also changing the ordering of the fields
        // of the Rust U128 struct.
        public readonly ulong Seconds;
        public readonly uint Nanoseconds;
    }
#pragma warning restore CA1815 // Override equals and operator equals on value types
    [ExposeToGuest(true)]
    public sealed class HyperLightExports
    {
        readonly StringWriter? writer;
        public HyperLightExports(StringWriter? writer)
        {
            this.writer = writer;
        }

#pragma warning disable CA1024 // Use properties where appropriate - Intentional as properties cannot be exposed to guest

        /// <summary>
        /// This is exposed by the GuestLibrary in function PrintMethod
        /// </summary>
        public int HostPrint(string message)
        {
            if (string.IsNullOrEmpty(message))
            {
                return 0;
            }
            if (this.writer != null)
            {
                writer.Write(message);
            }
            else
            {
                var oldColor = Console.ForegroundColor;
                Console.ForegroundColor = ConsoleColor.Green;
                Console.Write(message);
                Console.ForegroundColor = oldColor;
            }
            return message.Length;
        }

        /// <summary>
        /// This is required by os_thread_get_stack_boundary() in WAMR (which is needed to handle AOT compiled WASM)
        /// </summary>
        public static IntPtr GetStackBoundary()
        {
            //TODO: This should be implemented for Linux if we re-enable in process support on Linux.
            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                HyperlightException.LogAndThrowException<NotSupportedException>($"Hyperlight only supports in process execution on Windows.", MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
            }
            return new IntPtr((long)exports_get_stack_boundary());
        }

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
            var dur = exports_nanos_since_epoch();
            var nanos = dur.Nanoseconds + (dur.Seconds * 1_000_000_000);
            return (long)(nanos / 1000);
        }
#pragma warning restore CA1024 // Use properties where appropriate  

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong exports_get_stack_boundary();

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern Duration exports_nanos_since_epoch();

#pragma warning restore CA1707 // Remove the underscores from member name
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory


    }
}
