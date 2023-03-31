#pragma once
#include <stdint.h>
#include "hyperlight.h"

typedef struct
{
    uint64_t fbHostFunctionDetailsSize;
    void* fbHostFunctionDetails;
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
    uint64_t guest_function_dispatch_ptr;
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