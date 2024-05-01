#include "hyperlight.h"
#include <string.h>

// Force an entry in the PE file's relocation table to help us with testing PE file relocation
static int (*foo)(const char *format, ...) = printf;

uint8_t* simpleprintOutput(const char *message)
{
    int result = foo("%s", message);
    return GetFlatBufferResultFromInt(result);
}

uint8_t* setByteArrayToZero(const uint8_t* arrayPtr, int length)
{
    memset(arrayPtr, 0, length );
    return GetFlatBufferResultFromVoid();
}

uint8_t* printTwoArgs(const char* arg1, int arg2)
{
    size_t length = (size_t)strlen(arg1)  + 35;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d.", arg1, arg2);
    return printOutputAsGuestFunction(message);
}

uint8_t* printThreeArgs(const char* arg1, int arg2, int64_t arg3)
{
    size_t length = (size_t)strlen(arg1) + 61;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d.", arg1, arg2, arg3);
    return printOutputAsGuestFunction(message);
}

uint8_t* printFourArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4)
{
    size_t length = (size_t)strlen(arg1) + (size_t)strlen(arg4) + 67;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s.", arg1, arg2, arg3, arg4);
    return printOutputAsGuestFunction(message);
}

uint8_t* printFiveArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5)
{
    size_t length = (size_t)strlen(arg1) + (size_t)strlen(arg4) + (size_t)strlen(arg5) + 67;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s.", arg1, arg2, arg3, arg4, arg5);
    return printOutputAsGuestFunction(message);
}

void LogToHost(char* message, LogLevel level)
{
    LOG(level, message, "SimpleGuest");
}

uint8_t* printSixArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6)
{
    size_t length = (size_t)strlen(arg1) + (size_t)strlen(arg4) + (size_t)strlen(arg5) + 79;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s.", arg1, arg2, arg3, arg4, arg5, arg6 ? "true" : "false");
    return printOutputAsGuestFunction(message);
}

uint8_t* printSevenArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6, bool arg7)
{
    size_t length = (size_t)strlen(arg1) + (size_t)strlen(arg4) + (size_t)strlen(arg5) + 90;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s arg7:%s.", arg1, arg2, arg3, arg4, arg5, arg6 ? "true" : "false", arg7 ? "true" : "false");
    return printOutputAsGuestFunction(message);
}

uint8_t* printEightArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6, bool arg7, const char* arg8)
{
    size_t length = (size_t)strlen(arg1) + (size_t)strlen(arg4) + (size_t)strlen(arg5) + (size_t)strlen(arg8) + 96;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s arg7:%s arg8:%s.", arg1, arg2, arg3, arg4, arg5, arg6 ? "true" : "false", arg7 ? "true" : "false", arg8);
    return printOutputAsGuestFunction(message);
}

uint8_t* printNineArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6, bool arg7, const char* arg8, int64_t arg9)
{
    size_t length = (size_t)strlen(arg1) + (size_t)strlen(arg4) + (size_t)strlen(arg5) + (size_t)strlen(arg8) + 122;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s arg7:%s arg8:%s arg9:%d.", arg1, arg2, arg3, arg4, arg5, arg6 ? "true" : "false", arg7 ? "true" : "false", arg8, arg9);
    return printOutputAsGuestFunction(message);
}

uint8_t* printTenArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6, bool arg7, const char* arg8, int64_t arg9, int arg10)
{
    size_t length = (size_t)strlen(arg1) + (size_t)strlen(arg4) + (size_t)strlen(arg5) + (size_t)strlen(arg8) + 139;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s arg7:%s arg8:%s arg9:%d arg10:%d.", arg1, arg2, arg3, arg4, arg5, arg6 ? "true" : "false", arg7 ? "true" : "false", arg8, arg9, arg10);
    return printOutputAsGuestFunction(message);
}

uint8_t* stackAllocate(int length)
{
    if (0 == length)
    {
        length = GUEST_STACK_SIZE + 1;
    }
#pragma warning(suppress:6255)
    void* buffer = _alloca(length);

    return GetFlatBufferResultFromInt(length);;
}

uint8_t* bufferOverrun(const char* str)
{
    char buffer[17];
    size_t length = NULL == str ? 0 : strlen(str);

    // will overrun if length > 17;
    if (length > 0)
    {
        strncpy(buffer, str, length);
    }
    int result = (int) (17 - length);
    return GetFlatBufferResultFromInt(result);
}

// If the following two functions are compiled with /O2 then they get optimised away and the tests fail.s
#pragma optimize("",off)
#pragma warning(suppress:6262)
uint8_t* stackOverflow(int i)
{
    while (i-- != 0)
    {
        char nums[16384] = { i };
        stackOverflow(i);
    }
    return GetFlatBufferResultFromInt(i);
}

#pragma warning(suppress:6262)
uint8_t* largeVar()
{
    char buffer[GUEST_STACK_SIZE + 1] = { 0 };
    return GetFlatBufferResultFromInt(GUEST_STACK_SIZE + 1);
}

#pragma optimize("",on)

uint8_t* smallVar()
{
    char buffer[1024] = {0};
    return GetFlatBufferResultFromInt(1024);
}

uint8_t* callMalloc(int size)
{
    void* heapMemory = malloc(size);
    if (NULL == heapMemory)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    return GetFlatBufferResultFromInt(size);
}

uint8_t* mallocAndFree(int size)
{
    void* heapMemory = malloc(size);
    if (NULL == heapMemory)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    free(heapMemory);
    return GetFlatBufferResultFromInt(size);
}

uint8_t* echo(const char* msg)
{
    return GetFlatBufferResultFromString(msg);
}

uint8_t* getSizePrefixedBuffer(const void* data, uint32_t length) 
{
    return GetFlatBufferResultFromSizePrefixedBuffer((void*)data,length);
}

// Keep the CPU 100% busy forever
uint8_t* spin()
{
    while (true)
    {
    }
    return GetFlatBufferResultFromVoid();
}

uint8_t* printUsingPrintf(const char* msg)
{
    printf("%s", msg);
    return GetFlatBufferResultFromVoid();
}

uint8_t* guestAbortWithCode(uint32_t code)
{
    abort_with_code(code);
    return GetFlatBufferResultFromVoid();
}

uint8_t* guestAbortWithMessage(uint32_t code, const char* message)
{
    abort_with_code_and_message(code, message);
    return GetFlatBufferResultFromVoid();
}

uint8_t* executeOnStack()
{
    const uint8_t hlt = 0xF4;
    ((void (*)()) & hlt)();
    return GetFlatBufferResultFromVoid();
}

uint8_t *GuestDispatchFunction(ns(FunctionCall_table_t) functionCall)
{
    // this function is only used for a specific dotnet test.
    // The test checks the stack behavior of the input/output buffer
    // by calling the host before serializing the function call.
    // If the stack is not working correctly, the input or output buffer will be 
    // overwritten before the function call is serialized, and we will not be able
    // to verify that the function call name is "ThisIsNotARealFunction"

    LogToHost("Hi this is a log message that will overwrite the shared buffer if the stack is not working correctly", INFORMATION);
    
    int host_res = printOutput("Hi this is a log message that will overwrite the shared buffer if the stack is not working correctly");
    
    // verify that the function call is same format as expected
    flatbuffers_string_t name = Hyperlight_Generated_FunctionCall_function_name_get(functionCall);
    Hyperlight_Generated_Parameter_vec_t params = Hyperlight_Generated_FunctionCall_parameters_get(functionCall);
    size_t param_len = Hyperlight_Generated_Parameter_vec_len(params);
    Hyperlight_Generated_FunctionCallType_enum_t call_type = Hyperlight_Generated_FunctionCall_function_call_type_get(functionCall);
    Hyperlight_Generated_ReturnType_enum_t return_type = Hyperlight_Generated_FunctionCall_expected_return_type_get(functionCall);

    if (strcmp(name, "ThisIsNotARealFunctionButTheNameIsImportant") != 0 
        || param_len != 0
        || call_type != Hyperlight_Generated_FunctionCallType_guest
        || return_type != Hyperlight_Generated_ReturnType_hlint
        || host_res != 100)
    {
        // the function call got overwritten
        setError(GUEST_FUNCTION_NOT_FOUND, "FunctionDoesntExist");
        return GetFlatBufferResultFromInt(-1);
    }

    return GetFlatBufferResultFromInt(99);
}

GENERATE_FUNCTION(simpleprintOutput,1,hlstring);
GENERATE_FUNCTION(stackAllocate, 1, hlint);
GENERATE_FUNCTION(stackOverflow, 1, hlint);
GENERATE_FUNCTION(bufferOverrun, 1, hlstring);
GENERATE_FUNCTION(largeVar, 0);
GENERATE_FUNCTION(smallVar, 0);
GENERATE_FUNCTION(callMalloc, 1, hlint);
GENERATE_FUNCTION(mallocAndFree, 1, hlint);
GENERATE_FUNCTION(printTwoArgs, 2, hlstring, hlint);
GENERATE_FUNCTION(printThreeArgs, 3, hlstring, hlint, hllong);
GENERATE_FUNCTION(printFourArgs, 4, hlstring, hlint, hllong, hlstring);
GENERATE_FUNCTION(printFiveArgs, 5, hlstring, hlint, hllong, hlstring, hlstring);
GENERATE_FUNCTION(printSixArgs, 6, hlstring, hlint, hllong, hlstring, hlstring, hlbool);
GENERATE_FUNCTION(printSevenArgs, 7, hlstring, hlint, hllong, hlstring, hlstring, hlbool, hlbool);
GENERATE_FUNCTION(printEightArgs, 8, hlstring, hlint, hllong, hlstring, hlstring, hlbool, hlbool, hlstring);
GENERATE_FUNCTION(printNineArgs, 9, hlstring, hlint, hllong, hlstring, hlstring, hlbool, hlbool, hlstring, hllong);
GENERATE_FUNCTION(printTenArgs, 10, hlstring, hlint, hllong, hlstring, hlstring, hlbool, hlbool, hlstring, hllong, hlint);
GENERATE_FUNCTION(setByteArrayToZero, 2, hlvecbytes, hlint);
GENERATE_FUNCTION(echo, 1, hlstring);
GENERATE_FUNCTION(getSizePrefixedBuffer, 2, hlvecbytes, hlint);
GENERATE_FUNCTION(spin, 0);
GENERATE_FUNCTION(printUsingPrintf, 1, hlstring);
GENERATE_FUNCTION(guestAbortWithCode, 1, hlint);
GENERATE_FUNCTION(guestAbortWithMessage, 2, hlint, hlstring);
GENERATE_FUNCTION(executeOnStack, 0);


void HyperlightMain()
{
    RegisterFunction(FUNCTIONDETAILS("PrintOutput", simpleprintOutput));
    RegisterFunction(FUNCTIONDETAILS("StackAllocate", stackAllocate));
    RegisterFunction(FUNCTIONDETAILS("StackOverflow", stackOverflow));
    RegisterFunction(FUNCTIONDETAILS("BufferOverrun", bufferOverrun));
    RegisterFunction(FUNCTIONDETAILS("LargeVar", largeVar));
    RegisterFunction(FUNCTIONDETAILS("SmallVar", smallVar));
    RegisterFunction(FUNCTIONDETAILS("CallMalloc", callMalloc));
    RegisterFunction(FUNCTIONDETAILS("MallocAndFree", mallocAndFree));
    RegisterFunction(FUNCTIONDETAILS("PrintTwoArgs", printTwoArgs));
    RegisterFunction(FUNCTIONDETAILS("PrintThreeArgs", printThreeArgs));
    RegisterFunction(FUNCTIONDETAILS("PrintFourArgs", printFourArgs));
    RegisterFunction(FUNCTIONDETAILS("PrintFiveArgs", printFiveArgs));
    RegisterFunction(FUNCTIONDETAILS("PrintSixArgs", printSixArgs));
    RegisterFunction(FUNCTIONDETAILS("PrintSevenArgs", printSevenArgs));
    RegisterFunction(FUNCTIONDETAILS("PrintEightArgs", printEightArgs));
    RegisterFunction(FUNCTIONDETAILS("PrintNineArgs", printNineArgs));
    RegisterFunction(FUNCTIONDETAILS("PrintTenArgs", printTenArgs));
    RegisterFunction(FUNCTIONDETAILS("SetByteArrayToZero", setByteArrayToZero));
    RegisterFunction(FUNCTIONDETAILS("Echo", echo));
    RegisterFunction(FUNCTIONDETAILS("GetSizePrefixedBuffer", getSizePrefixedBuffer));
    RegisterFunction(FUNCTIONDETAILS("Spin", spin));
    RegisterFunction(FUNCTIONDETAILS("PrintUsingPrintf", printUsingPrintf));
    RegisterFunction(FUNCTIONDETAILS("GuestAbortWithCode", guestAbortWithCode));
    RegisterFunction(FUNCTIONDETAILS("GuestAbortWithMessage", guestAbortWithMessage));
    RegisterFunction(FUNCTIONDETAILS("ExecuteOnStack", executeOnStack));
}
