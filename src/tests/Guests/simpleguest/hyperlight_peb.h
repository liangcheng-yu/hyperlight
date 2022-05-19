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
    HostFunctionDefinitions hostFunctionDefinitions;
    HostException hostException;
    GuestError guestError;
    char* pCode;
    void* pOutb;
    InputData inputdata;
    OutputData outputdata;
    GuestHeapData guestheapData;
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


typedef struct
{
    char* FunctionName;
    uint64_t argc;
    uint64_t** argv;

} GuestFunctionCall;


typedef struct
{
    char* FunctionName;
    uint64_t** argv;

} HostFunctionCall;
