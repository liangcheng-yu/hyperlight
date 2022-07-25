using System;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Native
{
    using OpName = String;
    
    static class Syscall
    {        
        public static int CheckReturnVal(
            OpName opName,
            Func<int> fn,
            int expectedReturnVal
        ) {
            return CheckReturnVal(
                opName,
                fn,
                (int retVal) => retVal == expectedReturnVal
            );
        }

        public static int CheckReturnVal(
            OpName opName,
            Func<int> fn,
            Func<int, bool> checkRetVal
        ) {
            var ret = fn();
            if(!checkRetVal(ret)) {
                int error = Marshal.GetLastPInvokeError();
                throw new HyperlightException($"${opName}: Expected return value, got {ret} Pinvoke Last Error:{error}");
            }
            return ret;
        }

        public static uint CheckReturnVal(
            OpName opName,
            Func<uint> fn,
            uint expectedReturnVal
        ) {
            return CheckReturnVal(
                opName,
                fn,
                (uint retVal) => retVal == expectedReturnVal
            );
        }

        public static uint CheckReturnVal(
            OpName opName,
            Func<uint> fn,
            Func<uint, bool> checkRetVal
        ) {
            var ret = fn();
            if(!checkRetVal(ret)) {
                throw new HyperlightException($"${opName}: Expected return value, got {ret}");
            }
            return ret;
        }
    };
}
