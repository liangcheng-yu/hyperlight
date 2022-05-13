#pragma once
#include <stdint.h>

//TODO: Optmise this so it doesnt use fixed amounts of memory.

typedef struct
{
    uint64_t errorNo;
    uint64_t messageSize;
    char message[256];

} GuestError;

typedef struct
{
    uint8_t funcs[4096];
    uint8_t hostException[4096];
    GuestError error;
    uint8_t pCode[8];
    uint8_t pOutb[8];
    uint8_t input[65536];
    uint8_t output[65536];
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
