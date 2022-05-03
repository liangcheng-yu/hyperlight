#include <stdbool.h>
#include <stdarg.h>
#include "hyperlight_peb.h"
#include "hyperlight_error.h"

HyperlightPEB* pPeb;
typedef int (*guestFunc)(char*);
struct FuncEntry
{
    char* pFuncName;
    guestFunc pFunc;
};

bool runningHyperlight = true;
void (*outb_ptr)(uint16_t port, uint8_t value) = NULL;
const uint8_t get_rsi[] = { 0x48, 0x8b, 0xc6, 0xc3 };
const uint8_t get_rdi[] = { 0x48, 0x8b, 0xc7, 0xc3 };
const uint8_t set_rsi[] = { 0x48, 0x8b, 0xf1, 0xc3 };
const uint8_t set_rdi[] = { 0x48, 0x8b, 0xf9, 0xc3 };

#pragma optimize("", off)

uint64_t getrsi()
{
    ((uint64_t(*)(void)) get_rsi)();
}

uint64_t getrdi()
{
    ((uint64_t(*)(void)) get_rdi)();
}

void setrsi(uint64_t rsi)
{
    ((void(*)(uint64_t)) set_rsi)(rsi);
}

void setrdi(uint64_t rdi)
{
    ((void(*)(uint64_t)) set_rdi)(rdi);
}

void outb(uint16_t port, uint8_t value)
{
    const uint8_t outb[] = {0x89, 0xd0, 0x89, 0xca, 0xee, 0xc3};

    if (runningHyperlight)
        ((void (*)(uint16_t, uint8_t))outb)(port, value);
    else if (NULL != outb_ptr)
    {
        // We were passed a function pointer for outb - Use it

        // If we are running under Linux, it means the outb_ptr callback is
        // implemented by dotnet running on Linux.  In this case, the calling conventions
        // allow the dotnet code to overwrite rsi/rdi.  If this binary is built
        // using MSVC, it expects rsi/rdi to be preserved by anything it calls.  The optimizer
        // might make use of one of these registers, so we will save/restore them ourselves.
        uint64_t rsi = getrsi();
        uint64_t rdi = getrdi();
        outb_ptr(port, value);
        setrsi(rsi);
        setrdi(rdi);
    }
}

char* strncpy(char* dest, const char* src, size_t len)
{
    char* result = dest;
    while (len--)
    {
        *dest++ = *src++;
    }
    *dest = 0;
    return result;
}

int strcmp(char string1[], char string2[])
{
    for (int i = 0;; i++) {
        if (string1[i] != string2[i]) {
            return string1[i] < string2[i] ? -1 : 1;
        }

        if (string1[i] == '\0') {
            return 0;
        }
    }
}

char* strncat(char* dest, const char* src, int length )
{
    char* result = dest;
    dest += strlen(dest);
    int count = 0;

    while (++count <= length)
        *dest++ = *src++;
            
    *(++dest) = '0';
    return result;
}

int sendMessagetoHostMethod(char* methodName, char* guestMessage, char* message)
{
    char* messageToHost = strncat(guestMessage, message, strlen(message));
    return native_symbol_thunk(methodName, messageToHost);
}

int guestFunction(char *message)
{
    
    char guestMessage[256] = "Hello from GuestFunction, ";
    sendMessagetoHostMethod("HostMethod", guestMessage, message);
}

int guestFunction1(char* message)
{
    char guestMessage[256] = "Hello from GuestFunction1, ";
    sendMessagetoHostMethod("HostMethod1", guestMessage, message);
}

int printOutput(const char* message)
{
    int result = strlen(message);
    strncpy((void*)&pPeb->output, (void*)message, result);
    outb(100, 0);
    return result;
}

struct FuncEntry funcTable[] = {
    {"GuestMethod", &guestFunction},
    {"GuestMethod1", &guestFunction1},
    {"PrintOutput", &printOutput},
    {NULL, NULL}};

int strlen(const char* str)
{
    if (NULL == str)
    {
        return 0;
    }
    const char* s;
    for (s = str; *s; ++s)
        ;
    return (s - str);
}
void resetError()
{
    pPeb->error.errorNo = 0;
    *pPeb->error.message = NULL;
}

void setError(uint64_t errorCode, char* message)
{
    pPeb->error.errorNo = errorCode;
    int length = strlen(message);
    if (length >= sizeof(pPeb->error.message))
    {
        length = sizeof(pPeb->error.message);
    }

    if (length == 0)
    {
        *pPeb->error.message = NULL;
    }
    else
    {
        strncpy(pPeb->error.message, message, length);
    }

    *(uint32_t*)&pPeb->output = -1;
}

// Prevents compiler inserted function from generating Memory Access exits when calling alloca.
void __chkstk()
{
}

static void
halt()
{
    const uint8_t hlt = 0xF4;
    if (runningHyperlight)
        ((void (*)()) & hlt)();
}

void DispatchFunction()
{
    resetError();

    GuestFunctionCall *funcCall = &pPeb->output;
    
    if (NULL == funcCall->FunctionName)
    {
        setError(GUEST_FUNCTION_NAME_NOT_PROVIDED, NULL);
        goto halt;
    }

    guestFunc pFunc = NULL;

    for (uint32_t i = 0; funcTable[i].pFuncName != NULL; i++)
    {
        if (strcmp(funcCall->FunctionName, funcTable[i].pFuncName) == 0)
        {
            pFunc = funcTable[i].pFunc;
            break;
        }
    }

    if (NULL == pFunc)
    {
        setError(GUEST_FUNCTION_NOT_FOUND, funcCall->FunctionName);
        goto halt;
    }

    if (funcCall->argc == 0)
    {
        setError(GUEST_FUNCTION_PARAMETERS_MISSING, NULL);
        goto halt;
    }

    char* param;

    // TODO: Handle multiple parameters and ints
    // only processes the first argument if is a string , otherwise returns -1
    // for (uint32_t i = 0; i < funcCall->argc; i++)

    for (uint32_t i = 0; i < 1; i++)
    {
        uint64_t arg64 = (funcCall->argv + (8*i));
        // arg is a string
        if (arg64 & 0x8000000000000000)
        {
            param = (char*)(arg64 &= 0x7FFFFFFFFFFFFFFF);
        }
        // arg is an int
        else
        {
            setError(UNSUPPORTED_PARAMETER_TYPE, "0");
            goto halt;
        }
    }
    
    *(uint32_t *)&pPeb->output = pFunc(param);

halt:

    halt();
}

int native_symbol_thunk(char* functionName, ...)
{

    HostFunctionCall* functionCall = (HostFunctionCall*)&pPeb->output;
    functionCall->FunctionName = functionName;
    uint64_t* ptr = &functionCall->argv;

    va_list ap;

    va_start(ap, functionName);

    void* arg;

    while (arg = va_arg(ap, void*))
    {
        *ptr++ = arg;
    }

    va_end(ap);

    // TODO: Why is the return code getting output via outb?
    // This only happens if running in Hyperlight and on KVM.

    outb(101, 0);
    return *(int*)&pPeb->input;
}



long entryPoint(uint64_t pebAddress, int a, int b, int c)
{
    pPeb = (HyperlightPEB*)pebAddress;
    int result = 0;
    if (NULL == pPeb)
    {
        return -1;
    }
    else
    {
        resetError();
        if (NULL != *((const char*)&pPeb->code))
        {
            if (*((const char*)&pPeb->code) != 'J')
            {
                setError(CODE_HEADER_NOT_SET, NULL);
                goto halt;
            }
        }
        else
        {
            if (*((const char**)&pPeb->pCode)[0] != 'J')
            {
                setError(CODE_HEADER_NOT_SET, NULL);
                goto halt;
            }
        }

        // Either in WHP partition (hyperlight) or in memory.  If in memory, outb_ptr will be non-NULL

        outb_ptr = *(void**)pPeb->pOutb;
        if (outb_ptr)
            runningHyperlight = false;

        HostFunctions* pFuncs = pPeb->funcs;
        pFuncs->header.DispatchFunction = (uint64_t)DispatchFunction;

        if (!pFuncs->header.DispatchFunction)
        {
            setError(DISPATCH_FUNCTION_POINTER_NOT_SET, NULL);
            goto halt;
        }
    }

    // Setup return values
    *(uint32_t *)&pPeb->output = result;

halt:
    halt(); // This is a nop if we were just loaded into memory
    return result;
}