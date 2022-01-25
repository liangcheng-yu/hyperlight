#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdarg.h>
#include <string.h>


bool runningHyperlight = true;
bool runningAsExe = false;
void (*outb_ptr)(uint16_t port, uint8_t value) = NULL;
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
        int BUFFER_SIZE = 128;
        char* buffer = (char*)_alloca(BUFFER_SIZE);
        vsprintf_s(buffer, BUFFER_SIZE, format, args);
        result = strlen(buffer);
        strcpy_s((char *)0x220000, BUFFER_SIZE, buffer);
        outb(100, 0);
    }
    va_end(args);
    return result;
}

#pragma optimize("", on)

int main(int argc, char *argv[])
{
    if (argc > 1 && argv[1] != NULL)
    {
        return printOutput("Hello, %s!!\n", argv[1]);
    }
    return printOutput("Hello, World!!\n");
}

long entryPoint()
{
    int result = 0;
    if (*((const char *)0x230000) == 'M')
    {
        // We were run as a normal EXE
        runningHyperlight = false;
        runningAsExe = true;
        mainCRTStartup();
    }
    else
    {
        // TODO: Populate the args.

        int argc = 0;
        char **argv = NULL;

        // Either in WHP partition (hyperlight) or in memory.  If in memory, outb_ptr will be non-NULL
        outb_ptr = *(void **)(0x210000 - 16);
        if (outb_ptr)
            runningHyperlight = false;
        result = main(argc, argv);
    }

    // For non-EXE, cpy return value to memory
    if (!runningAsExe)
    {
        // Setup return values
        *(uint32_t *)0x220000 = result;
        halt(); // This is a nop if we are running as an EXE or if we were just loaded into memory
    }

    return result;
}