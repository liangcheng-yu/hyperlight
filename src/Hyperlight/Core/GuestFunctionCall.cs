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
    public struct GuestArgument
    {
        public ParameterKind argt;
        public ulong argv;
    }
    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    public struct GuestFunctionCall
    {
        public ulong pFunctionName;
        public ulong argc;
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 10)]
        public GuestArgument[] guestArguments;
    }

}
