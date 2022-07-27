#pragma once
#include <stdint.h>
#include <stdbool.h>
#include "printf.h"

#define calloc               dlcalloc
#define free                 dlfree
#define malloc               dlmalloc
#define realloc              dlrealloc
#define GUEST_ERROR          15

#define GENERATE_FUNCTION(function, paramsc, ... ) GENERATE_FUNCTION_EXPANDED(GENERATE_FUNCTION_##paramsc##, (function, __VA_ARGS__))

#define GENERATE_FUNCTION_EXPANDED(function ,args_expanded) function args_expanded

#define GENERATE_FUNCTION_0(function, ...) \
int __call_##function(Parameter* params) \
{ return function();} \
int __##function##_pcount = 0; \
ParameterKind __##function##_pKind[] = { 0 };

#define GENERATE_FUNCTION_1(function, union0_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member);} \
int __##function##_pcount = 1;\
ParameterKind __##function##_pKind[] = {union0_member};

#define GENERATE_FUNCTION_2(function, union0_member, union1_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member, params[1].value.union1_member);} \
int __##function##_pcount = 2;\
ParameterKind __##function##_pKind[] = {union0_member, union1_member};

#define GENERATE_FUNCTION_3(function, union0_member, union1_member, union2_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member);} \
int __##function##_pcount = 3; \
ParameterKind __##function##_pKind[] = {union0_member, union1_member, union2_member};

#define GENERATE_FUNCTION_4(function, union0_member, union1_member, union2_member, union3_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member);} \
int __##function##_pcount = 4; \
ParameterKind __##function##_pKind[] = {union0_member,union1_member, union2_member, union3_member};

#define GENERATE_FUNCTION_5(function, union0_member, union1_member, union2_member, union3_member, union4_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member);} \
int __##function##_pcount = 5; \
ParameterKind __##function##_pKind[] = {union0_member, union1_member, union2_member, union3_member, union4_member};

#define GENERATE_FUNCTION_6(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member);} \
int __##function##_pcount = 6; \
ParameterKind __##function##_pKind[] = {union0_member, union1_member, union2_member, union3_member, union4_member, union5_member};

#define GENERATE_FUNCTION_7(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member, params[6].value.union6_member);} \
int __##function##_pcount = 7; \
ParameterKind __##function##_pKind[] = {union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member};

#define GENERATE_FUNCTION_8(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member, union7_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member, params[6].value.union6_member, params[7].value.union7_member);} \
int __##function##_pcount = 8; \
ParameterKind __##function##_pKind[] = {union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member, union7_member};

#define GENERATE_FUNCTION_9(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member, union7_member, union8_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member, params[6].value.union6_member, params[7].value.union7_member, params[8].value.union8_member);} \
int __##function##_pcount = 9; \
ParameterKind __##function##_pKind[] = {union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member, union7_member, union8_member};

#define GENERATE_FUNCTION_10(function, union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member, union7_member, union8_member, union9_member) \
int __call_##function(Parameter* params) \
{ return function(params[0].value.union0_member, params[1].value.union1_member, params[2].value.union2_member, params[3].value.union3_member, params[4].value.union4_member, params[5].value.union5_member, params[6].value.union6_member, params[7].value.union7_member, params[8].value.union8_member, params[9].value.union9_member);} \
int __##function##_pcount = 10; \
ParameterKind __##function##_pKind[] = {union0_member, union1_member, union2_member, union3_member, union4_member, union5_member, union6_member, union7_member, union8_member, union9_member};

#define FUNCTIONDETAILS(function) &__call_##function, __##function##_pcount, __##function##_pKind

typedef enum {
    i32,
    i64,
    string,
    boolean,
} ParameterKind;

typedef struct
{
    union
    {
        int32_t i32;
        int64_t i64;
        const char* string;
        bool boolean;
    } value;
    ParameterKind kind;
} Parameter;

typedef int (*guestFunc)(Parameter* params);

typedef struct FuncEntry
{
    const char* FunctionName;
    guestFunc pFunction;
    int paramCount;
    ParameterKind* parameterKind;
} FuncEntry;

typedef struct {
    char* functionName;
    int32_t paramc;
    Parameter* paramv;
} GuestFunctionDetails;


typedef struct
{
    char* FunctionName;
    char* FunctionSignature;
    uint64_t Flags;
} HostFunctionDefinition;

typedef struct
{
    uint64_t CountOfFunctions;
    HostFunctionDefinition* HostFunctionDefinitions;
    
} HostFunctionDetails;

int printOutput(const char*);

void HyperlightMain();

void setError(uint64_t, char*);

void* dlmalloc(size_t);

void  dlfree(void*);

void* dlrealloc(void*, size_t);

void* dlcalloc(size_t, size_t);

char* strncpy(char*, const char*, size_t);

size_t dlmalloc_set_footprint_limit(size_t bytes);

void RegisterFunction(const char*, guestFunc, int, ParameterKind[]);

HostFunctionDetails* GetHostFunctionDetails();

int GuestDispatchFunction(GuestFunctionDetails*);

int native_symbol_thunk(char*, ...);