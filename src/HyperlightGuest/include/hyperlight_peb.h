#pragma once
#include <stdint.h>
#include "hyperlight.h"

typedef struct
{
    uint64_t functionDefinitionSize;
    void* functionDefinitions;
} HostFunctionDefinitions;

typedef struct
{
    uint64_t hostExceptionSize;

} HostException;

typedef struct
{
    uint64_t inputDataSize;
    void* inputDataBuffer;

} InputData;

typedef struct
{
    uint64_t outputDataSize;
    void* outputDataBuffer;

} OutputData;

typedef struct
{
    uint64_t guestHeapSize;
    void* guestHeapBuffer;
} GuestHeapData;

typedef struct
{
    uint64_t minStackAddress;

} GuestStackData;

typedef struct
{
    uint64_t security_cookie_seed;
    HostFunctionDefinitions hostFunctionDefinitions;
    HostException hostException;
    void* pGuestErrorBuffer;
    uint64_t guestErrorBufferSize;
    char* pCode;
    void* pOutb;
    InputData inputdata;
    OutputData outputdata;
    GuestHeapData guestheapData;
    GuestStackData gueststackData;
} HyperlightPEB;

typedef struct
{
    uint64_t CountOfFunctions;
    uint64_t DispatchFunction;
} HostFunctionHeader;

typedef struct
{
    HostFunctionHeader header;
} HostFunctions;

typedef struct
{
    ParameterKind argt;
    uint64_t argv;

} GuestArgument;

typedef struct
{
    char* FunctionName;
    uint64_t argc;
    GuestArgument guestArguments[];

} GuestFunctionCall;

typedef struct
{
    char* FunctionName;
    uint64_t* argv;

} HostFunctionCall;