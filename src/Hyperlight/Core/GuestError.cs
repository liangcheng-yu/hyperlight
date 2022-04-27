using System;
using System.Runtime.InteropServices;

namespace Hyperlight.Core
{
    enum GuestErrorCode : ulong
    {
        NO_ERROR                            = 0,  // The function call was successful
        CODE_HEADER_NOT_SET                 = 1,   // The expected PE header was not found in the Guest Binary
        UNSUPPORTED_PARAMETER_TYPE          = 2,   // The type of the parameter is not supported by the Guest.
        GUEST_FUNCTION_NAME_NOT_PROVIDED    = 3,  // The Guest function name was not provided by the host.  
        GUEST_FUNCTION_NOT_FOUND            = 4,  // The function does not exist in the Guest.  
        GUEST_FUNCTION_PARAMETERS_MISSING   = 5,   // Parameters are missing for the guest function.
        DISPATCH_FUNCTION_POINTER_NOT_SET   = 6,  // Host Call Dispatch Function Pointer is not present.
        OUTB_ERROR                          = 7,   // Error in OutB Function
    }


    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    internal class GuestError
    {
        public GuestErrorCode ErrorCode;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)]
        public string Message;
    }

    [Serializable]
    public class HyperlightException : Exception
    {
        public HyperlightException(string message) : base(message)
        {
        }

        public HyperlightException(string message, Exception innerException) : base(message, innerException)
        {
        }

        protected HyperlightException(System.Runtime.Serialization.SerializationInfo serializationInfo, System.Runtime.Serialization.StreamingContext streamingContext) : base(serializationInfo, streamingContext)    
        {
        }

        public HyperlightException()
        {
        }
    }
}
