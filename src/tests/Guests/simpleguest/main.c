#include <stdbool.h>
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
uint64_t getrsi();
uint64_t getrdi();
void setrsi(uint64_t rsi);
void setrdi(uint64_t rsi);

#pragma optimize("", off)
void outb(uint16_t port, uint8_t value)
{
    const uint8_t outb[] = { 0x89, 0xd0, 0x89, 0xca, 0xee, 0xc3 };

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

// Prevents compiler inserted function from generating Memory Access exits when calling alloca. 
// TODO: need to figure out if this needs a real implementation.
void
__chkstk()
{}

static void
halt()
{
    const uint8_t hlt = 0xF4;
    if (runningHyperlight)
        ((void (*)()) & hlt)();
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

int printOutput(const char* message)
{
    int result = strlen(message);
    strncpy((void*)&pPeb->output, (void*)message, result);
    outb(100, 0);
    return result;
}

struct FuncEntry funcTable[] = {
    {"PrintOutput", &printOutput},
    {NULL, NULL} };

int strlen(const char* str)
{
    const char* s;
    for (s = str; *s; ++s)
        ;
    return (s - str);
}

void setError(uint64_t errorCode)
{
    pPeb->error = errorCode;
}

void DispatchFunction()
{
    setError(NO_ERROR);
    GuestFunctionCall* funcCall = &pPeb->output;

    if (NULL == funcCall->FunctionName)
    {
        setError(GUEST_FUNCTION_NAME_NOT_PROVIDED);
        *(uint32_t*)&pPeb->output = -1;
        return;
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
        setError(GUEST_FUNCTION_NOT_FOUND);
        *(uint32_t*)&pPeb->output = -1;
        return;
    }

    if (funcCall->argc == 0)
    {
        setError(GUEST_FUNCTION_PARAMETERS_NOT_FOUND);
        *(uint32_t*)&pPeb->output = -1;
        return;
    }

    char* param;

    // TODO: Handle multiple parameters and ints
    // only processes the first argument if is a string , otherwise returns -1
    // for (uint32_t i = 0; i < funcCall->argc; i++)

    for (uint32_t i = 0; i < 1; i++)
    {
        uint64_t arg64 = (funcCall->argv + (8 * i));
        // arg is a string
        if (arg64 & 0x8000000000000000)
        {
            param = (char*)(arg64 &= 0x7FFFFFFFFFFFFFFF);
        }
        // arg is an int
        else
        {
            setError(UNSUPPORTED_PARAMETER_TYPE);
            *(uint32_t*)&pPeb->output = -1;
        }
    }

    *(uint32_t*)&pPeb->output = pFunc(param);

    halt();
}

#pragma optimize("", on)

int entryPoint(uint64_t pebAddress)
{
    pPeb = (HyperlightPEB*)pebAddress;
    int result = 0;
    if (NULL == pPeb)
    {
        return -1;
    }
    else
    {
        if (NULL != *((const char*)&pPeb->code))
        {
            if (*((const char*)&pPeb->code) != 'J')
            {
                setError(CODE_HEADER_NOT_SET);
                return -1;
            }
        }
        else
        {
            if (*((const char**)&pPeb->pCode)[0] != 'J')
            {
                setError(CODE_HEADER_NOT_SET);
                return -1;
            }
        }

        // Either in WHP partition (hyperlight) or in memory.  If in memory, outb_ptr will be non-NULL
        outb_ptr = *(void**)pPeb->pOutb;
        if (outb_ptr)
            runningHyperlight = false;
        HostFunctions* pFuncs = pPeb->funcs;
        pFuncs->header.DispatchFunction = (uint64_t)DispatchFunction;
    }

    // Setup return values
    *(int32_t*)&pPeb->output = result;
    halt(); // This is a nop if we were just loaded into memory

    return;
}