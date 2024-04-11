#pragma once

//TODO: When we get rid of the C Guest Library we should remove this file entirely and generate it with bindgen.

#include <stdint.h>
#include <stdbool.h>
#include "printf.h"
#include "function_call_builder.h"
#include "function_call_reader.h"
#include "function_types_builder.h"
#include "function_types_reader.h"
#include "guest_function_definition_builder.h"
#include "guest_function_definition_reader.h"
#include "guest_log_data_builder.h"
#include "host_function_definition_builder.h"
#include "host_function_definition_reader.h"
#include "host_function_details_builder.h"
#include "host_function_details_reader.h"
#include "function_call_result_builder.h"
#include "function_call_result_reader.h"

#define ns(x) FLATBUFFERS_WRAP_NAMESPACE(Hyperlight_Generated, x) 

#define GUEST_ERROR          15

#ifdef USING_RUST_GUEST

void* hlmalloc(size_t);
void* hlcalloc(size_t);
void* hlrealloc(void*, size_t);
void hlfree(void*);
void abort();
void abort_with_code(int);

#define free                 hlfree
#define malloc               hlmalloc
#define calloc               hlcalloc
#define realloc              hlrealloc

#else

#define calloc               dlcalloc
#define free                 dlfree
#define malloc               dlmalloc
#define realloc              dlrealloc

#endif


#define GENERATE_FUNCTION(function, paramsc, ... ) GENERATE_FUNCTION_##paramsc(function, __VA_ARGS__)

#define GENERATE_FUNCTION_0(function, ...) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ return function();} \
int __##function##_pcount = 0; \
ns(ParameterType_enum_t) __##function##_pKind[] = { 0 };

#define GENERATE_FUNCTION_1(function, union0_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[1] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member);\
} \
int __##function##_pcount = 1;\
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member)};

#define GENERATE_FUNCTION_2(function, union0_member, union1_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[2] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member, params[1].value.union1_member); \
} \
int __##function##_pcount = 2;\
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member), ns(ParameterType_##union1_member)};

#define GENERATE_FUNCTION_3(function, union0_member, union1_member, union2_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[3] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member); \
} \
int __##function##_pcount = 3; \
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member), ns(ParameterType_##union1_member), ns(ParameterType_##union2_member)};

#define GENERATE_FUNCTION_4(function, union0_member, union1_member, union2_member, union3_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[4] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member); \
} \
int __##function##_pcount = 4; \
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member),ns(ParameterType_##union1_member), ns(ParameterType_##union2_member), ns(ParameterType_##union3_member)};

#define GENERATE_FUNCTION_5(function, union0_member, union1_member, union2_member, union3_member, union4_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[5] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member); \
} \
int __##function##_pcount = 5; \
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member), ns(ParameterType_##union1_member), ns(ParameterType_##union2_member), ns(ParameterType_##union3_member), ns(ParameterType_##union4_member)};

#define GENERATE_FUNCTION_6(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[6] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member); \
} \
int __##function##_pcount = 6; \
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member), ns(ParameterType_##union1_member), ns(ParameterType_##union2_member), ns(ParameterType_##union3_member), ns(ParameterType_##union4_member), ns(ParameterType_##union5_member)};

#define GENERATE_FUNCTION_7(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[7] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member, params[6].value.union6_member); \
} \
int __##function##_pcount = 7; \
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member), ns(ParameterType_##union1_member), ns(ParameterType_##union2_member), ns(ParameterType_##union3_member), ns(ParameterType_##union4_member), ns(ParameterType_##union5_member), ns(ParameterType_##union6_member)};

#define GENERATE_FUNCTION_8(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member, union7_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[8] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member, params[6].value.union6_member, params[7].value.union7_member); \
} \
int __##function##_pcount = 8; \
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member), ns(ParameterType_##union1_member), ns(ParameterType_##union2_member), ns(ParameterType_##union3_member), ns(ParameterType_##union4_member), ns(ParameterType_##union5_member), ns(ParameterType_##union6_member), ns(ParameterType_##union7_member)};

#define GENERATE_FUNCTION_9(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member, union7_member, union8_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[9] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member, params[6].value.union6_member, params[7].value.union7_member, params[8].value.union8_member); \
} \
int __##function##_pcount = 9; \
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member), ns(ParameterType_##union1_member), ns(ParameterType_##union2_member), ns(ParameterType_##union3_member), ns(ParameterType_##union4_member), ns(ParameterType_##union5_member), ns(ParameterType_##union6_member), ns(ParameterType_##union7_member), ns(ParameterType_##union8_member)};

#define GENERATE_FUNCTION_10(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member, union7_member, union8_member, union9_member) \
uint8_t* __call_##function(ns(FunctionCall_table_t) functionCall) \
{ \
    Parameter params[10] = {0}; \
    GetFunctionCallParameters(functionCall, params);   \
    return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member, params[6].value.union6_member, params[7].value.union7_member, params[8].value.union8_member, params[9].value.union9_member); \
} \
int __##function##_pcount = 10; \
ns(ParameterType_enum_t) __##function##_pKind[] = {ns(ParameterType_##union0_member), ns(ParameterType_##union1_member), ns(ParameterType_##union2_member), ns(ParameterType_##union3_member), ns(ParameterType_##union4_member), ns(ParameterType_##union5_member), ns(ParameterType_##union6_member), ns(ParameterType_##union7_member), ns(ParameterType_##union8_member), ns(ParameterType_##union9_member)};

#define FUNCTIONDETAILS(name, function)  CreateFunctionDefinition(name, &__call_##function, __##function##_pcount, __##function##_pKind)
typedef enum {
    hlint,
    hllong,
    hlstring,
    hlbool,
    hlvecbytes
} ParameterKind;

typedef struct
{
    union
    {
        int32_t hlint;
        int64_t hllong;
        const char* hlstring;
        bool hlbool;
        const uint8_t* hlvecbytes;
    } value;
    ParameterKind kind;
} Parameter;

typedef uint8_t* (*guestFunc)(ns(FunctionCall_table_t) functionCall);
typedef enum LogLevel
{
    TRACE = 0,
    DEBUG = 1,
    INFORMATION = 2,
    WARNING = 3,
    ERROR = 4,
    CRTICAL = 5,
    NONE = 6
} LogLevel;

typedef struct LogData
{
    char* Message;
    char* Source;
    LogLevel Level;
    char* Caller;
    char* SourceFile;
    int32_t Line;
} LogData;

uint8_t* printOutputAsGuestFunction(const char*);

int printOutput(const char*);

void HyperlightMain();

void setError(uint64_t, char*);

void* dlmalloc(size_t);

void  dlfree(void*);

void* dlrealloc(void*, size_t);

void* dlcalloc(size_t, size_t);

size_t dlmalloc_set_footprint_limit(size_t bytes);

void RegisterFunction(ns(GuestFunctionDefinition_ref_t));

ns(HostFunctionDetails_table_t) GetHostFunctionDetails();

uint8_t* GuestDispatchFunction(ns(FunctionCall_table_t));

void native_symbol_thunk(char*, int32_t, ...);

int native_symbol_thunk_returning_int(char*, int32_t, ...);

void CallHostFunction(char* functionName, int32_t, va_list ap);

long long GetHostReturnValueAsLongLong();

int GetHostReturnValueAsInt();

unsigned long long GetHostReturnValueAsULongLong();

unsigned int GetHostReturnValueAsUInt();

void Log(LogLevel logLevel, const char* message, const char* source, const char* caller, const char* sourceFile, int32_t line);

#define LOG(loglevel, message, source) Log(loglevel,message,source, __func__,__FILE__,__LINE__)

unsigned int GetOSPageSize();

uint8_t* GetStackBoundary();

long long GetHyperLightTickCount();

long long GetTimeSinceBootMicrosecond();

void GetFunctionCallParameters(ns(FunctionCall_table_t) functionCall, Parameter params[]);

uint8_t* GetFlatBufferResultFromInt(uint32_t);

uint8_t* GetFlatBufferResultFromString(const char*);

uint8_t* GetFlatBufferResultFromVoid();

uint8_t* GetFlatBufferResultFromSizePrefixedBuffer(void*, int32_t);

ns(GuestFunctionDefinition_ref_t) CreateFunctionDefinition(const char*, guestFunc, int, ns(ParameterType_enum_t)[]);

void abort_with_code_and_message(uint32_t, const char*);
void __halt();