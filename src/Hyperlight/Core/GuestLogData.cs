using System;
using System.Runtime.InteropServices;
using Microsoft.Extensions.Logging;

namespace Hyperlight.Core
{
    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    internal struct GuestLogData
    {
        IntPtr message;
        IntPtr source;
        readonly byte level;
        IntPtr caller;
        IntPtr sourceFile;
        readonly int line;

        internal string Message => Marshal.PtrToStringAnsi(message) ?? string.Empty;

        internal string Source => Marshal.PtrToStringAnsi(source) ?? string.Empty;

        internal LogLevel LogLevel => (LogLevel)level;

        internal string Caller => Marshal.PtrToStringAnsi(caller) ?? "unknown";

        internal string SourceFile => Marshal.PtrToStringAnsi(sourceFile) ?? "unknown";

        internal int Line => line > 0 ? line : -1;
        internal static GuestLogData Create(IntPtr address, long offset)
        {
            var logData = Marshal.PtrToStructure<GuestLogData>(address);
            if (offset > 0)
            {
                logData.message = new IntPtr(logData.message.ToInt64() + offset);
                logData.source = new IntPtr(logData.source.ToInt64() + offset);
                logData.caller = new IntPtr(logData.caller.ToInt64() + offset);
                logData.sourceFile = new IntPtr(logData.sourceFile.ToInt64() + offset);
            }
            return logData;
        }
    }
}
