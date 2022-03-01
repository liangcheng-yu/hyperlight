#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdarg.h>
#include <string.h>
#include "hyperlight_peb.h"

bool runningHyperlight = true;
bool runningAsExe = false;
int BUFFER_SIZE = 256;

HyperlightPEB* pPeb;

void (*outb_ptr)(uint16_t port, uint8_t value) = NULL;
typedef int (*guestFunc)(char *);
struct FuncEntry
{
    char *pFuncName;
    guestFunc pFunc;
};

uint64_t getrsi();
uint64_t getrdi();
void setrsi(uint64_t rsi);
void setrdi(uint64_t rsi);

extern int mainCRTStartup(void);

#pragma optimize("", off)

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

int printOutput(const char *format, ...)
{
    int result = 0;
    va_list args = NULL;
    va_start(args, format);

    if (runningAsExe)
    {
        result = vprintf(format, args);
    }
    else
    {
        char *buffer = (char *)_alloca(BUFFER_SIZE);
        vsprintf_s(buffer, BUFFER_SIZE, format, args);
        result = strlen(buffer);
        strcpy_s((char *)&pPeb->output, BUFFER_SIZE, buffer);
        outb(100, 0);
    }
    va_end(args);
    return result;
}

int guestFunction(char *message)
{
    if (NULL != message)
    {
        return printOutput("Hello from GuestFunction, %s!!.\n", message);
    }

    return printOutput("Hello, World!! from Guest Function\n");
}

struct FuncEntry funcTable[] = {
    {"GuestMethod", &guestFunction},
    {NULL, NULL}};

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

    GuestFunctionCall *funcCall = &pPeb->output;
    // TODO: How to return error details?
    
    if (NULL == funcCall->FunctionName)
    {
        printOutput("No function name found in DispatchFunction.\n");
        *(uint32_t *)&pPeb->output = -1;
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
        printOutput("Function %s not found in FunctionTable.\n", funcCall->FunctionName);
        *(uint32_t *)&pPeb->output = -1;
        return;
    }

    if (funcCall->argc == 0)
    {
        printOutput("No parameters found\n");
        *(uint32_t *)&pPeb->output = -1;
        return;
    }

    char* param;

    // TODO: Handle multiple parameters and ints
    // only processes the first argument if is not a string then convert to string
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
            char* buffer = (char*)_alloca(BUFFER_SIZE);
            sprintf_s(buffer, BUFFER_SIZE, "%d", (uint32_t)arg64);
            param = buffer;
        }
    }
    

    *(uint32_t *)&pPeb->output = pFunc(param);

    halt();
}

int native_symbol_thunk(char *functionName, void *a, void *b, void *c, void *d)
{

    HostFunctionCall* functionCall = (HostFunctionCall*)&pPeb->output;
    functionCall->FunctionName = functionName;
    uint64_t* ptr = &functionCall->argv;

    *ptr = a;
    ptr++;
    *ptr = b;
    ptr++;
    *ptr = c;
    ptr++;
    *ptr = d;

    // TODO: Why is the return code getting output via outb?
    // This only happens if runing in Hyperlight and on KVM.

    outb(101, 0);
    return *(int *)&pPeb->input;
}

#pragma optimize("", on)

int main(int argc, char *argv[])
{
    if (!runningAsExe)
    {
        char *message;
        if (argc > 1 && argv[1] != NULL)
        {
            message = (char *)_alloca(BUFFER_SIZE);
            sprintf_s(message, BUFFER_SIZE, "Hello, %s!!", argv[1]);
        }
        else
        {
            message = "Hello, World!!";
        }
        return native_symbol_thunk("HostMethod", message, NULL, NULL, NULL);
    }

    if (argc > 1 && argv[1] != NULL)
    {
        return printOutput("Hello, %s!!\n", argv[1]);
    }

    return printOutput("Hello, World!!\n");
}

long entryPoint(uint64_t pebAddress, int a, int b, int c)
{

    pPeb = (HyperlightPEB*)pebAddress;
    int result = 0;
    if (NULL == pPeb)
    {
        // We were run as a normal EXE
        runningHyperlight = false;
        runningAsExe = true;
        mainCRTStartup();
    }
    else
    {
        if (*((const char*)&pPeb->code) != 'J')
        {
            return -1;
        }
        // TODO: Populate the args.

        int argc = 0;
        char **argv = NULL;

        // Either in WHP partition (hyperlight) or in memory.  If in memory, outb_ptr will be non-NULL
        
        outb_ptr = *(void **)pPeb->pOutb;
        if (outb_ptr)
            runningHyperlight = false;

        HostFunctions* pFuncs = pPeb->funcs;
        pFuncs->header.DispatchFunction = (uint64_t)DispatchFunction;
        result = main(argc, argv);
    }

    // For non-EXE, cpy return value to memory
    if (!runningAsExe)
    {
        // Setup return values
        *(uint32_t *)&pPeb->output = result;
        halt(); // This is a nop if we are running as an EXE or if we were just loaded into memory
    }

    return result;
}