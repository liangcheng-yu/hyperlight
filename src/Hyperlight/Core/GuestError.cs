using System;
using System.Runtime.InteropServices;

namespace Hyperlight.Core
{
    enum GuestErrorCode : ulong
    {
        NO_ERROR = 0,  // The function call was successful
        CODE_HEADER_NOT_SET = 1,   // The expected PE header was not found in the Guest Binary
        UNSUPPORTED_PARAMETER_TYPE = 2,   // The type of the parameter is not supported by the Guest.
        GUEST_FUNCTION_NAME_NOT_PROVIDED = 3,  // The Guest function name was not provided by the host.  
        GUEST_FUNCTION_NOT_FOUND = 4,  // The function does not exist in the Guest.  
        GUEST_FUNCTION_PARAMETERS_MISSING = 5,   // Parameters are missing for the guest function.
        DISPATCH_FUNCTION_POINTER_NOT_SET = 6,  // Host Call Dispatch Function Pointer is not present.
        OUTB_ERROR = 7,   // Error in OutB Function
        UNKNOWN_ERROR = 8,   // The guest error is unknown.
    }


    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    internal class GuestError
    {
        internal GuestErrorCode ErrorCode;
        internal ulong MaxMessageSize;
        internal string Message;

        internal GuestError(GuestErrorCode guestErrorCode, int maxMessageSize, string message)
        {
            if (message.Length >= maxMessageSize)
            {
                message = message.Substring(0, maxMessageSize - 1);
            }
            Message = message;
            ErrorCode = guestErrorCode;
            MaxMessageSize = (ulong)maxMessageSize;
        }
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
