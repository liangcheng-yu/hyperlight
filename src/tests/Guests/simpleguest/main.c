#include "hyperlight.h"

int simpleprintOutput(const char* message)
{
    return printOutput(message);
}

int setByteArrayToZero(void* arrayPtr, int length)
{
    while (length--)
    {
        *(((char*)arrayPtr)++) = 0;
    }
    return length;
}

int printTwoArgs(const char* arg1, int arg2)
{
    size_t length = strlen(arg1)  + 35;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d.", arg1, arg2);
    return printOutput(message);
}

int printThreeArgs(const char* arg1, int arg2, int64_t arg3)
{
    size_t length = strlen(arg1) + 61;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d.", arg1, arg2, arg3);
    return printOutput(message);
}

int printFourArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4)
{
    size_t length = strlen(arg1) + strlen(arg4) + 67;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s.", arg1, arg2, arg3, arg4);
    return printOutput(message);
}

int printFiveArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5)
{
    size_t length = strlen(arg1) + strlen(arg4) + strlen(arg5) + 67;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s.", arg1, arg2, arg3, arg4, arg5);
    return printOutput(message);
}

int printSixArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6)
{
    size_t length = strlen(arg1) + strlen(arg4) + strlen(arg5) + 79;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s.", arg1, arg2, arg3, arg4, arg5, arg6 ? "True" : "False");
    return printOutput(message);
}

int printSevenArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6, bool arg7)
{
    size_t length = strlen(arg1) + strlen(arg4) + strlen(arg5) + 90;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s arg7:%s.", arg1, arg2, arg3, arg4, arg5, arg6 ? "True" : "False", arg7 ? "True" : "False");
    return printOutput(message);
}

int printEightArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6, bool arg7, const char* arg8)
{
    size_t length = strlen(arg1) + strlen(arg4) + strlen(arg5) + strlen(arg8) + 96;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s arg7:%s arg8:%s.", arg1, arg2, arg3, arg4, arg5, arg6 ? "True" : "False", arg7 ? "True" : "False", arg8);
    return printOutput(message);
}

int printNineArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6, bool arg7, const char* arg8, int64_t arg9)
{
    size_t length = strlen(arg1) + strlen(arg4) + strlen(arg5) + strlen(arg8) + 122;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s arg7:%s arg8:%s arg9:%d.", arg1, arg2, arg3, arg4, arg5, arg6 ? "True" : "False", arg7 ? "True" : "False", arg8, arg9);
    return printOutput(message);
}

int printTenArgs(const char* arg1, int arg2, int64_t arg3, const char* arg4, const char* arg5, bool arg6, bool arg7, const char* arg8, int64_t arg9, int arg10)
{
    size_t length = strlen(arg1) + strlen(arg4) + strlen(arg5) + strlen(arg8) + 139;
    char* message = malloc(length);
    if (NULL == message)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    snprintf(message, length, "Message: arg1:%s arg2:%d arg3:%d arg4:%s arg5:%s arg6:%s arg7:%s arg8:%s arg9:%d arg10:%d.", arg1, arg2, arg3, arg4, arg5, arg6 ? "True" : "False", arg7 ? "True" : "False", arg8, arg9, arg10);
    return printOutput(message);
}

int stackAllocate(int length)
{
    if (0 == length)
    {
        length = GUEST_STACK_SIZE + 1;
    }
    void* buffer = _alloca(length);

    return length;
}

int bufferOverrun(const char* str)
{
    char buffer[17];
    size_t length = NULL == str ? 0 : strlen(str);

    // will overrun if length > 17;
    if (length > 0)
    {
        strncpy(buffer, str, length);
    }
    return (int) (17 - length);
}

int stackOverflow(int i)
{
    while (i-- != 0)
    {
        char nums[16384] = { i };
        stackOverflow(i);
    }
    return i;
}

int largeVar()
{
    char buffer[GUEST_STACK_SIZE + 1] = { 0 };
    return GUEST_STACK_SIZE + 1;
}

int smallVar()
{
    char buffer[1024] = {0};
    return 1024;
}

int callMalloc(int size)
{
    void* heapMemory = malloc(size);
    if (NULL == heapMemory)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    return size;
}

int mallocAndFree(int size)
{
    void* heapMemory = malloc(size);
    if (NULL == heapMemory)
    {
        setError(GUEST_ERROR, "Malloc Failed");
    }
    free(heapMemory);
    return size;
}

GENERATE_FUNCTION(simpleprintOutput, 1, string);
GENERATE_FUNCTION(stackAllocate, 1, i32);
GENERATE_FUNCTION(stackOverflow, 1, i32);
GENERATE_FUNCTION(bufferOverrun, 1, string);
GENERATE_FUNCTION(largeVar, 0);
GENERATE_FUNCTION(smallVar, 0);
GENERATE_FUNCTION(callMalloc, 1, i32);
GENERATE_FUNCTION(mallocAndFree, 1, i32);
GENERATE_FUNCTION(printTwoArgs, 2, string, i32);
GENERATE_FUNCTION(printThreeArgs, 3, string, i32, i64);
GENERATE_FUNCTION(printFourArgs, 4, string, i32, i64, string);
GENERATE_FUNCTION(printFiveArgs, 5, string, i32, i64, string, string);
GENERATE_FUNCTION(printSixArgs, 6, string, i32, i64, string, string, boolean);
GENERATE_FUNCTION(printSevenArgs, 7, string, i32, i64, string, string, boolean, boolean);
GENERATE_FUNCTION(printEightArgs, 8, string, i32, i64, string, string, boolean, boolean, string);
GENERATE_FUNCTION(printNineArgs, 9, string, i32, i64, string, string, boolean, boolean, string, i64);
GENERATE_FUNCTION(printTenArgs, 10, string, i32, i64, string, string, boolean, boolean, string, i64, i32);
GENERATE_FUNCTION(setByteArrayToZero, 2, bytearray, i32);

void HyperlightMain()
{
    RegisterFunction("PrintOutput", FUNCTIONDETAILS(simpleprintOutput));
    RegisterFunction("StackAllocate", FUNCTIONDETAILS(stackAllocate));
    RegisterFunction("StackOverflow", FUNCTIONDETAILS(stackOverflow));
    RegisterFunction("BufferOverrun", FUNCTIONDETAILS(bufferOverrun));
    RegisterFunction("LargeVar", FUNCTIONDETAILS(largeVar));
    RegisterFunction("SmallVar", FUNCTIONDETAILS(smallVar));
    RegisterFunction("CallMalloc", FUNCTIONDETAILS(callMalloc));
    RegisterFunction("MallocAndFree", FUNCTIONDETAILS(mallocAndFree)); 
    RegisterFunction("PrintTwoArgs", FUNCTIONDETAILS(printTwoArgs));
    RegisterFunction("PrintThreeArgs", FUNCTIONDETAILS(printThreeArgs));
    RegisterFunction("PrintFourArgs", FUNCTIONDETAILS(printFourArgs));
    RegisterFunction("PrintFiveArgs", FUNCTIONDETAILS(printFiveArgs));
    RegisterFunction("PrintSixArgs", FUNCTIONDETAILS(printSixArgs));
    RegisterFunction("PrintSevenArgs", FUNCTIONDETAILS(printSevenArgs));
    RegisterFunction("PrintEightArgs", FUNCTIONDETAILS(printEightArgs));
    RegisterFunction("PrintNineArgs", FUNCTIONDETAILS(printNineArgs));
    RegisterFunction("PrintTenArgs", FUNCTIONDETAILS(printTenArgs));
    RegisterFunction("SetByteArrayToZero", FUNCTIONDETAILS(setByteArrayToZero));
}
