#pragma once
#include <stdint.h>

typedef struct
{
    uint8_t padding1[4096];
    uint8_t pml4[4096];
    uint8_t pdtp[4096];
    uint8_t pd[4096];
    uint8_t funcs[4096];
    uint8_t padding2[45040];
    uint8_t pOutb[16];
    uint8_t input[65536];
    uint8_t output[65536];
    uint8_t code;
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