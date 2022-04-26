#pragma once
#define NO_ERROR                            0   // The function call was successful
#define CODE_HEADER_NOT_SET                 1   // The expected PE header was not found in the Guest Binary
#define UNSUPPORTED_PARAMETER_TYPE          2   // The type of the parameter is not supported by the Guest.
#define GUEST_FUNCTION_NAME_NOT_PROVIDED    3   // The Guest function name was not provided by the host.  
#define GUEST_FUNCTION_NOT_FOUND            4   // The function does not exist in the Guest.  
#define GUEST_FUNCTION_PARAMETERS_NOT_FOUND 5   // Parameters are missing for the guest function.
#define DISPATCH_FUNCTION_POINTER_NOT_SET   6   // Host Call Dispatch Function Pointer is not present.