using System;
using System.Runtime.InteropServices;

namespace Hyperlight.Core
{
    public enum ParameterKind : Int32
    {
        i32,
        i64,
        str,
        boolean,
    }
    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
#pragma warning disable CA1815 // Override equals and operator equals on value types
    public struct GuestArgument
#pragma warning restore CA1815 // Override equals and operator equals on value types
    {
        public ParameterKind argt;
        public ulong argv;
    }
    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
#pragma warning disable CA1815 // Override equals and operator equals on value types
    public struct GuestFunctionCall
#pragma warning restore CA1815 // Override equals and operator equals on value types
    {
        public ulong pFunctionName;
        public ulong argc;
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = Constants.MAX_NUMBER_OF_GUEST_FUNCTION_PARAMETERS)]
        public GuestArgument[] guestArguments;
    }

}
