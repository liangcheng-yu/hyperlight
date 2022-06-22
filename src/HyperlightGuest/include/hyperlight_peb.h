#pragma once
#include <stdint.h>

typedef struct
{
    uint64_t errorNo;
    uint64_t messageSize;
    char*  message;

} GuestError;

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
    GuestError guestError;
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
    char* FunctionName;
    char* FunctionSignature;
    uint64_t Flags;
} FunctionDefinition;

typedef struct
{
    HostFunctionHeader header;
} HostFunctions;

typedef enum {
    i32,
    i64,
    string,
    boolean,
} ParameterKind;

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
    uint64_t** argv;

} HostFunctionCall;
