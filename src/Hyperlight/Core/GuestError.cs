using System;

namespace Hyperlight.Core
{
    enum GuestErrorCode : long
    {
        NO_ERROR = 0,  // The function call was successful
        CODE_HEADER_NOT_SET = 1,  // The expected PE header was not found in the Guest Binary
        UNSUPPORTED_PARAMETER_TYPE = 2,  // The type of the parameter is not supported by the Guest.
        GUEST_FUNCTION_NAME_NOT_PROVIDED = 3,  // The Guest function name was not provided by the host.  
        GUEST_FUNCTION_NOT_FOUND = 4,  // The function does not exist in the Guest.  
        GUEST_FUNCTION_INCORRECT_NO_OF_PARAMETERS = 5,  // Parameters are missing for the guest function.
        DISPATCH_FUNCTION_POINTER_NOT_SET = 6,  // Host Call Dispatch Function Pointer is not present.
        OUTB_ERROR = 7,  // Error in OutB Function
        UNKNOWN_ERROR = 8,  // The guest error is unknown.
        STACK_OVERFLOW = 9,  // Guest stack allocations caused stack overflow
        GS_CHECK_FAILED = 10,  // __security_check_cookie failed
        TOO_MANY_GUEST_FUNCTIONS = 11,  // The guest tried to register too many guest functions
        FAILURE_IN_DLMALLOC = 12,  // this error is set when dlmalloc calls ABORT (e.g. function defined in #define ABORT (dlmalloc_abort() calls setError with this errorcode)
        MALLOC_FAILED = 13,  // this error is set when malloc returns 0 bytes.
        GUEST_FUNCTION_PARAMETER_TYPE_MISMATCH = 14,  // The function call parameter type was not the expected type.  
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
