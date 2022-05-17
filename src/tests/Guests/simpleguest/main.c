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

void resetError()
{
    pPeb->guestError.errorNo = 0;
    *pPeb->guestError.message = NULL;
}

void setError(uint64_t errorCode, char* message)
{
    pPeb->guestError.errorNo = errorCode;
    int length = strlen(message);
    if (length >= pPeb->guestError.messageSize)
    {
        length = pPeb->guestError.messageSize - 1;
    }

    if (length == 0)
    {
        *pPeb->guestError.message = NULL;
    }
    else
    {
        strncpy(pPeb->guestError.message, message, length);
    }

    *(uint32_t*)&pPeb->output = -1;
}

void DispatchFunction()
{
    resetError();
    GuestFunctionCall* funcCall = &pPeb->output;

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
    // only processes the first argument if is a string ,otherwise returns -1

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
            setError(UNSUPPORTED_PARAMETER_TYPE, "0");
            goto halt;
        }
    }

    *(uint32_t*)&pPeb->output = pFunc(param);

halt:

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
        resetError();

        if (*pPeb->pCode != 'J')
        {
            setError(CODE_HEADER_NOT_SET, NULL);
            goto halt;
        }

        // Either in WHP partition (hyperlight) or in memory.  If in memory, outb_ptr will be non-NULL
        outb_ptr = pPeb->pOutb;
        if (outb_ptr)
            runningHyperlight = false;
        HostFunctions* pFuncs = pPeb->guestFunctionDefinition.functionDefinitions;
        pFuncs->header.DispatchFunction = (uint64_t)DispatchFunction;
    }

    // Setup return values
    *(int32_t*)&pPeb->output = result;

halt:

    halt(); // This is a nop if we were just loaded into memory
    return;
}