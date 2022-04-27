using System;

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
            var ret = fn();
            if(ret != expectedReturnVal) {
                throw new Exception($"${opName}: Expected return value {expectedReturnVal}, got {ret}");
            }
            return ret;
        }

        public static uint CheckReturnVal(
            OpName opName,
            Func<uint> fn,
            uint expectedReturnVal
        ) {
            var ret = fn();
            if(ret != expectedReturnVal) {
                throw new Exception($"${opName}: Expected return value {expectedReturnVal}, got {ret}");
            }
            return ret;
        }
    };
}
