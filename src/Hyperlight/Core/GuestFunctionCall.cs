using System;
using System.Runtime.InteropServices;

namespace Hyperlight.Core
{
#pragma warning disable CA1028 // If possible, make the underlying type of ParameterKind System.Int32 instead of ushort
    public enum ParameterKind : ushort
#pragma warning restore CA1028 // If possible, make the underlying type of ParameterKind System.Int32 instead of ushort
    {
        i32,
        i64,
        str,
        boolean,
        bytearray,
        none,
    }
    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
#pragma warning disable CA1815 // Override equals and operator equals on value types
    public struct GuestArgument
#pragma warning restore CA1815 // Override equals and operator equals on value types
    {
        [MarshalAs(UnmanagedType.U2)]
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
