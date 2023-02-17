
#include "hyperlight.h" 
#include "hyperlight_guest.h"
#include "hyperlight_error.h"
#include "hyperlight_peb.h"
#include <setjmp.h>
#include <string.h>
#include "guest_error_builder.h"
#include "guest_error_reader.h"

#define ns(x) FLATBUFFERS_WRAP_NAMESPACE(Hyperlight_Generated, x) 

extern int _fltused = 0;
uintptr_t __security_cookie;
jmp_buf jmpbuf;
HyperlightPEB* pPeb;
FuncTable* funcTable;
bool runningInHyperlight = true;
void (*outb_ptr)(uint16_t port, uint8_t value) = NULL;

void RegisterFunction(const char* FunctionName, guestFunc pFunction, int paramCount, ParameterKind parameterKind[])
{
    if (funcTable->next > funcTable->size)
    {
        char message[100] = { 0 };
        snprintf(message, 100, "Function Table Limit is %d.", funcTable->size);
        setError(TOO_MANY_GUEST_FUNCTIONS, message);
    }
    FuncEntry* funcEntry = (FuncEntry*)calloc(1, sizeof(FuncEntry));
    if (NULL == funcEntry)
    {
        setError(MALLOC_FAILED, NULL);
    }

#pragma warning(suppress:6011)
    funcEntry->FunctionName = FunctionName;
    funcEntry->pFunction = pFunction;
    funcEntry->paramCount = paramCount;
    funcEntry->parameterKind = parameterKind;

    funcTable->funcEntry[funcTable->next] = funcEntry;
    funcTable->next++;
}

void InitialiseFunctionTable(int size)
{
    size = size > 0 ? size : DEFAULT_FUNC_TABLE_SIZE;
    funcTable = (FuncTable*)calloc(1, sizeof(FuncTable));
    if (NULL == funcTable)
    {
        setError(MALLOC_FAILED, NULL);
    }

#pragma warning(suppress:6011)
    funcTable->next = 0;
    funcTable->size = size;
    funcTable->funcEntry = (FuncEntry**)(calloc(size, sizeof(FuncEntry*)));
    if (NULL == funcTable->funcEntry)
    {
        setError(MALLOC_FAILED, NULL);
    }
}

void checkForHostError()
{
    size_t size;
    void* buffer = flatbuffers_read_size_prefix(pPeb->pGuestErrorBuffer, &size);
    assert(NULL != buffer);
    // No need to free buffer as its just a pointer to an offset in the message buffer in the PEB
    ns(GuestError_table_t guestError = ns(GuestError_as_root(buffer)));
    if (size > 0)
    {
        ns(GuestError_table_t guestError = ns(GuestError_as_root(buffer)));
        if (ns(GuestError_code(guestError) != ns(ErrorCode_NoError)))
        {
            memset(pPeb->outputdata.outputDataBuffer, 0, pPeb->outputdata.outputDataSize);
            *(uint32_t*)pPeb->outputdata.outputDataBuffer = -1;
            longjmp(jmpbuf, 1);
        }
    }
}

void outb(uint16_t port, uint8_t value)
{
    if (runningInHyperlight)
    {
        hloutb(port, value);
    }
    else if (NULL != outb_ptr)
    {
        // We were passed a function pointer for outb - Use it

        // If we are running under Linux, it means the outb_ptr callback is
        // implemented by dotnet running on Linux.  In this case, the calling conventions
        // allow the dotnet code to overwrite rsi/rdi.  If this binary is built
        // using MSVC, it expects rsi/rdi to be preserved by anything it calls.  The optimizer
        // might make use of one of these registers, so we will save/restore them ourselves.

        // This code is not needed at present as in process calls are disabled on Linux.
        // However, it is left here in case we need to re-enable in process calls in future.
        // TODO: Enable if Linux in process is supported.
        //uint64_t rsi = getrsi();
        //uint64_t rdi = getrdi();
        outb_ptr(port, value);
        //setrsi(rsi);
        //setrdi(rdi);
    }

    // TODO: The following code should be uncommented when OUTB_ABORT is handled correctly in the Host.

    // If we are aborting just exit
    /*if (port != OUTB_ABORT)
    {
        longjmp(jmpbuf, 1);  
    }*/
    checkForHostError();
}

#pragma optimize("", off)

void halt()
{
    const uint8_t hlt = 0xF4;
    if (runningInHyperlight)
    {
        ((void (*)()) & hlt)();
    }
}

#pragma optimize("", on)

void writeError(uint64_t errorCode, char* message)
{
    // Create a flatbuffer builder object
    flatcc_builder_t builder;

    // Initialize the builder object.
    flatcc_builder_init(&builder);

    // Validate the error code

    ns(ErrorCode_enum_t) code = ns(ErrorCode_UnknownError);

    if (ns(ErrorCode_is_known_value((ns(ErrorCode_enum_t))errorCode)))
    {
        code = ((ns(ErrorCode_enum_t))errorCode);
    }

    // Create the flatbuffer
    ns(GuestError_start_as_root_with_size(&builder));
    ns(GuestError_code_add(&builder, code));
    if (NULL != message)
    {
        ns(GuestError_message_create(&builder, message, strlen(message)));
    }
    ns(GuestError_end_as_root(&builder));

    size_t flatb_size = flatcc_builder_get_buffer_size(&builder);
    assert(flatb_size <= pPeb->guestErrorBufferSize);

    // Write the flatbuffer to the guest error buffer

    assert(flatcc_builder_copy_buffer(&builder, (void*)pPeb->pGuestErrorBuffer, flatb_size));
}

void resetError()
{
    writeError(0, NULL);
}

// SetError sets the specified error and message in memory and then halts execution by returning the the point that setjmp was called

void setError(uint64_t errorCode, char* message)
{
    writeError(errorCode, message);
    memset(pPeb->outputdata.outputDataBuffer, 0, pPeb->outputdata.outputDataSize);
    *(uint32_t*)pPeb->outputdata.outputDataBuffer = -1;
    longjmp(jmpbuf, 1);
}

void CallHostFunction(char* functionName, va_list ap)
{
    memset(pPeb->outputdata.outputDataBuffer, 0, pPeb->outputdata.outputDataSize);
    HostFunctionCall* functionCall = (HostFunctionCall*)pPeb->outputdata.outputDataBuffer;
    functionCall->FunctionName = functionName;
    uint64_t** ptr = &functionCall->argv;

    uint64_t* arg;

    while ((arg = va_arg(ap, uint64_t*)))
    {
        *ptr++ = arg;
    }

    outb(OUTB_CALL_FUNCTION, 0);
}

// TODO: Make these functions generic.

// Calls a Host Function that returns an int
int native_symbol_thunk_returning_int(char* functionName, ...)
{

    va_list ap = NULL;

    va_start(ap, functionName);

    CallHostFunction(functionName, ap);

    va_end(ap);

    return GetHostReturnValueAsInt();
}

int GetHostReturnValueAsInt()
{
    return *((int*)pPeb->inputdata.inputDataBuffer);
}

// Calls a Host Function that returns an int
unsigned int native_symbol_thunk_returning_uint(char* functionName, ...)
{

    va_list ap = NULL;

    va_start(ap, functionName);

    CallHostFunction(functionName, ap);

    va_end(ap);

    return GetHostReturnValueAsUInt();
}

unsigned int GetHostReturnValueAsUInt()
{
    return *((unsigned int*)pPeb->inputdata.inputDataBuffer);
}

// Calls a Host Function that returns an long long
long long native_symbol_thunk_returning_longlong(char* functionName, ...)
{

    va_list ap = NULL;

    va_start(ap, functionName);

    CallHostFunction(functionName, ap);

    va_end(ap);

    return  GetHostReturnValueAsLongLong();
}

long long GetHostReturnValueAsLongLong()
{
    return *((long long*)pPeb->inputdata.inputDataBuffer);
}

// Calls a Host Function that returns an ulong long
unsigned long long native_symbol_thunk_returning_ulonglong(char* functionName, ...)
{

    va_list ap = NULL;

    va_start(ap, functionName);

    CallHostFunction(functionName, ap);

    va_end(ap);

    return  GetHostReturnValueAsULongLong();
}

unsigned long long GetHostReturnValueAsULongLong()
{
    return *((unsigned long long*)pPeb->inputdata.inputDataBuffer);
}

int CallGuestFunction(GuestFunctionDetails* guestfunctionDetails)
{
    guestFunc pFunction = NULL;
    int requiredParameterCount = 0;
    ParameterKind* parameterKind = NULL;

    for (int i = 0; i < funcTable->next; i++)
    {
        if (strcmp(guestfunctionDetails->functionName, funcTable->funcEntry[i]->FunctionName) == 0)
        {
            pFunction = funcTable->funcEntry[i]->pFunction;
            requiredParameterCount = funcTable->funcEntry[i]->paramCount;
            parameterKind = funcTable->funcEntry[i]->parameterKind;
            break;
        }
    }

    if (NULL == pFunction)
    {
        // If the function was not found call the GuestDispatchFunction method optionally provided by the guest program.
        // This allows functions exposed by a guest runtime (e.g. WASM) to be called.
        return GuestDispatchFunction(guestfunctionDetails);
    }

    if (requiredParameterCount != guestfunctionDetails->paramc)
    {
        char message[100] = { 0 };
        snprintf(message, 100, "Called function %s with %d parameters but it takes %d.", guestfunctionDetails->functionName, guestfunctionDetails->paramc, requiredParameterCount);
        setError(GUEST_FUNCTION_INCORRECT_NO_OF_PARAMETERS, message);
    }

    for (int i = 0; i < requiredParameterCount; i++)
    {
        if (guestfunctionDetails->paramv[i].kind != parameterKind[i])
        {
            char message[100] = { 0 };
            snprintf(message, 100, "Function %s parameter %d.", guestfunctionDetails->functionName, guestfunctionDetails->paramc);
            setError(GUEST_FUNCTION_PARAMETER_TYPE_MISMATCH, message);
        }
    }

    return pFunction(guestfunctionDetails->paramv);
}

void DispatchFunction()
{
    GuestFunctionDetails* guestFunctionDetails = NULL;
    Parameter* paramv = NULL;
    uint64_t* parg_to_free = NULL;
    // setjmp is used to capture state so that if an error occurs then lngjmp is called in setError and control returns to this point , the if returns false and the program exits/halts
    if (!setjmp(jmpbuf))
    {
        resetError();
        GuestFunctionCall* funcCall = (GuestFunctionCall*)pPeb->outputdata.outputDataBuffer;

        if (NULL == funcCall->FunctionName)
        {
            setError(GUEST_FUNCTION_NAME_NOT_PROVIDED, NULL);
        }

        guestFunctionDetails = (GuestFunctionDetails*)calloc(1, sizeof(GuestFunctionDetails));

        if (NULL == guestFunctionDetails)
        {
            setError(MALLOC_FAILED, NULL);
        }

#pragma warning(suppress:6011)
        guestFunctionDetails->functionName = funcCall->FunctionName;
        guestFunctionDetails->paramc = (int32_t)funcCall->argc;

        if (0 == guestFunctionDetails->paramc)
        {
            guestFunctionDetails->paramv = NULL;
        }
        else
        {
            paramv = (Parameter*)calloc(guestFunctionDetails->paramc, sizeof(Parameter));
            if (NULL == paramv)
            {
                setError(MALLOC_FAILED, NULL);
            }
            guestFunctionDetails->paramv = paramv;
        }

        parg_to_free = calloc(guestFunctionDetails->paramc, sizeof(uint64_t));
        if (NULL == parg_to_free)
        {
            setError(MALLOC_FAILED, NULL);
        }

        bool nextParamIsLength = false;
        for (int32_t i = 0; i < guestFunctionDetails->paramc; i++)
        {
#pragma warning(suppress:6011)
            parg_to_free[i] = 0;
            GuestArgument guestArgument = funcCall->guestArguments[i];
            if (nextParamIsLength)
            {
                if (guestArgument.argt != i32)
                {
                    char message[15];
                    snprintf(message, 15, "Parameter %d", i);
                    setError(ARRAY_LENGTH_PARAM_IS_MISSING, message);
                }
                else
                {
                    uint32_t len = (uint32_t)guestArgument.argv;
                    void* ptr = calloc(1, len);
                    if (NULL == ptr)
                    {
                        setError(MALLOC_FAILED, NULL);
                    }
                    int32_t byteArrayIndex = i - 1;
                    GuestArgument  byteArrayArgument = funcCall->guestArguments[byteArrayIndex];
                    parg_to_free[i] = (uint64_t)ptr;
                    memcpy(ptr, (void*)byteArrayArgument.argv, (size_t)len);
                    guestFunctionDetails->paramv[byteArrayIndex].value.bytearray = (void*)ptr;
                    guestFunctionDetails->paramv[i].value.i32 = len;
                    guestFunctionDetails->paramv[i].kind = i32;
                    nextParamIsLength = false;
                }
            }
            else
            {
                switch (guestArgument.argt)
                {
                case (string):
                    // Length + 1 as we need to make sure the string is null terminated
                    size_t length = strlen((char*)guestArgument.argv) + 1;
                    void* ptr = (void*)calloc(1, length);
                    if (NULL == ptr)
                    {
                        setError(MALLOC_FAILED, NULL);
                    }
                    parg_to_free[i] = (uint64_t)ptr;
                    strncpy((char*)ptr, (char*)guestArgument.argv, length);
#pragma warning(suppress:28182)
                    guestFunctionDetails->paramv[i].value.string = (char*)ptr;
                    guestFunctionDetails->paramv[i].kind = string;
                    break;
                case (i32):
                    guestFunctionDetails->paramv[i].value.i32 = (uint32_t)guestArgument.argv;
                    guestFunctionDetails->paramv[i].kind = i32;
                    break;
                case (i64):
                    guestFunctionDetails->paramv[i].value.i64 = (uint64_t)guestArgument.argv;
                    guestFunctionDetails->paramv[i].kind = i64;
                    break;
                case (boolean):
                    guestFunctionDetails->paramv[i].value.boolean = (bool)guestArgument.argv;
                    guestFunctionDetails->paramv[i].kind = boolean;
                    break;
                case (bytearray):
                    guestFunctionDetails->paramv[i].kind = bytearray;
                    nextParamIsLength = true;
                    break;
                default:
                    setError(GUEST_FUNCTION_PARAMETER_TYPE_MISMATCH, NULL);
                    break;
                }
            }
        }
        if (nextParamIsLength)
        {
            setError(ARRAY_LENGTH_PARAM_IS_MISSING, "Last parameter should be the length of the array");
        }

        int result = CallGuestFunction(guestFunctionDetails);
        memset(pPeb->outputdata.outputDataBuffer, 0, pPeb->outputdata.outputDataSize);
        *((uint32_t*)(pPeb->outputdata.outputDataBuffer)) = result;

        for (int32_t i = 0; i < guestFunctionDetails->paramc; i++)
        {
            if (parg_to_free[i])
            {
                free((void*)parg_to_free[i]);
            }
        }
        free(parg_to_free);
    }

    free(guestFunctionDetails);
    free(paramv);

    halt();  // This is a nop if we were just loaded into memory
}

// this is required by print functions to write output.
// calls from print functions are buffered until
// either the output buffer is full or a null character is sent

void _putchar(char c)
{
    static int index = 0;
    if (index == 0)
    {
        memset(pPeb->outputdata.outputDataBuffer, 0, pPeb->outputdata.outputDataSize);
    }
    char* ptr = pPeb->outputdata.outputDataBuffer;

    if (index >= pPeb->outputdata.outputDataSize)
    {
        ptr[index] = '\0';
        outb(OUTB_WRITE_OUTPUT, 0);
        index = 0;
        ptr = pPeb->outputdata.outputDataBuffer;
    }

    if (c != '\0')
    {
        ptr[index++] = c;
        return;
    }

    ptr[index++] = c;
    ptr[index] = '\0';

    outb(OUTB_WRITE_OUTPUT, 0);
    index = 0;
}

bool CheckOutputBufferSize(char* dest, size_t size)
{
    return (uint64_t)dest - (uint64_t)pPeb->outputdata.outputDataBuffer + size > pPeb->outputdata.outputDataSize;
}

char* CheckAndCopyString(const char* source, char* dest)
{
    size_t length = NULL == source ? 0 : strlen(source);

    if (0 == length)
    {
        setError(GUEST_ERROR, "Length was zero");
    }

    if (CheckOutputBufferSize(dest, ++length))
    {
        setError(GUEST_ERROR, "Output buffer is full");
    }

#pragma warning(suppress : 4996)
    return strncpy(dest, (char*)source, length) + length;

}

void WriteLogData(LogLevel logLevel, const char* message, const char* source, const char* caller, const char* sourceFile, int32_t line)
{
    memset(pPeb->outputdata.outputDataBuffer, 0, pPeb->outputdata.outputDataSize);
    LogData* logData = (LogData*)pPeb->outputdata.outputDataBuffer;
    logData->Message = (char*)pPeb->outputdata.outputDataBuffer + sizeof(LogData);
    logData->Source = CheckAndCopyString(message, logData->Message);
    logData->Level = logLevel;
    logData->Caller = CheckAndCopyString(source, logData->Source);
    logData->SourceFile = CheckAndCopyString(caller, logData->Caller);
    CheckAndCopyString(sourceFile, logData->SourceFile);
    logData->Line = line;
}

void Log(LogLevel logLevel, const char* message, const char* source, const char* caller, const char* sourceFile, int32_t line)
{
    WriteLogData(logLevel, message, source, caller, sourceFile, line);
    outb(OUTB_LOG, 0);
}


// this is called when /Gs check fails

void report_gsfailure()
{
    setError(GS_CHECK_FAILED, NULL);
}

// Called by dlmalloc using ABORT

// TODO: Once we update the host to deal with aborts correctly we should update these handlers so that they return a code to explain what caused the abort
// i.e. they should be updated to call abort_with_code(ERROR_CODE) instead of abort()

void dlmalloc_abort()
{
    WriteLogData(CRTICAL, "dlmalloc_abort", "HyperLightGuest", __FUNCTION__, __FILE__, __LINE__);
    abort();
}

void __assert_fail(const char* expr, const char* file, int line, const char* func)
{
    size_t message_size = 256;
    char message[256] = { '0' };
    snprintf(message, message_size, "Assertion failed: %s ", expr);
    WriteLogData(CRTICAL, message, "HyperLightGuest", func, file, line);
    abort();
}

// Called by dlmalloc using ABORT

void dlmalloc_failure()
{
    Log(CRTICAL, "dlmalloc_failure", "HyperLightGuest", __FUNCTION__, __FILE__, __LINE__);
    abort();
}


void abort_with_code(uint32_t code)
{
    outb(OUTB_ABORT, code);
}

void abort()
{
    abort_with_code(0);
}

// this function is called by dlmalloc to allocate heap memory.

void* hyperlightMoreCore(size_t size)
{
    void* ptr = NULL;
    static void* unusedHeapBufferPointer = 0;
    static size_t allocated = 0;
    if (size > 0)
    {
        // Trying to use more memory than is available.
        // This should not happen if dlmalloc_set_footprint_limit was called with pPeb->guestheapData.guestHeapSize. 

        if (allocated + size > pPeb->guestheapData.guestHeapSize)
        {
            // TODO: Set an error message
            return (void*)MFAIL;
        }

        if (0 == unusedHeapBufferPointer)
        {
            ptr = (char*)pPeb->guestheapData.guestHeapBuffer;
        }
        else
        {
            ptr = (char*)unusedHeapBufferPointer;
        }

        allocated += size;
        unusedHeapBufferPointer = (char*)ptr + size;
        return ptr;
    }
    else if (size < 0)
    {
        // This should not happen according to dlmalloc docs as MORECORE_CANNOT_TRIM is set. 
        // TODO: Set an error message
        return (void*)MFAIL;
    }
    else
    {
        return unusedHeapBufferPointer;
    }
}

HostFunctionDetails* GetHostFunctionDetails()
{

    HostFunctionHeader* hostFunctionHeader = (HostFunctionHeader*)pPeb->hostFunctionDefinitions.functionDefinitions;
    size_t functionCount = hostFunctionHeader->CountOfFunctions;
    if (functionCount == 0)
    {
        return NULL;
    }

    HostFunctionDetails* hostFunctionDetails = (HostFunctionDetails*)calloc(1, sizeof(HostFunctionDetails));
    if (NULL == hostFunctionDetails)
    {
        setError(MALLOC_FAILED, NULL);
    }

#pragma warning(suppress:6011)
    hostFunctionDetails->CountOfFunctions = functionCount;
#pragma warning(suppress:6305) 
    hostFunctionDetails->HostFunctionDefinitions = (HostFunctionDefinition*)((char*)pPeb->hostFunctionDefinitions.functionDefinitions + sizeof(HostFunctionHeader));

    return hostFunctionDetails;
}

__declspec(safebuffers) int entryPoint(uint64_t pebAddress, uint64_t seed, int functionTableSize)
{

    // TODO: We should try and write to stderr here in case the program was started from the command line, note that we dont link against the CRT so we cant use printf
    pPeb = (HyperlightPEB*)pebAddress;
    if (NULL == pPeb)
    {
        return -1;
    }
    __security_init_cookie();
  
    // setjmp is used to capture state so that if an error occurs then lngjmp is called in setError and control returns to this point , the if returns false and the program exits/halts
    if (!setjmp(jmpbuf))
    {
        // Either in WHP partition (hyperlight) or in memory.  If in memory, outb_ptr will be non-NULL
        outb_ptr = (void(*)(uint16_t, uint8_t))pPeb->pOutb;
        if (outb_ptr)
            runningInHyperlight = false;
        HostFunctions* pFuncs = (HostFunctions*)pPeb->hostFunctionDefinitions.functionDefinitions;
        pFuncs->header.DispatchFunction = (uint64_t)DispatchFunction;

        dlmalloc_set_footprint_limit(pPeb->guestheapData.guestHeapSize);

        resetError();

        //TODO: Pass FunctionTablesize in entryPoint.

        InitialiseFunctionTable(0);

        // Call Hyperlightmain in the Guest , the guest can use this to register functions that can be called from the host.

        HyperlightMain();

        // Setup return values
        memset(pPeb->outputdata.outputDataBuffer, 0, pPeb->outputdata.outputDataSize);
        *(int32_t*)pPeb->outputdata.outputDataBuffer = 0;
    }

    halt(); // This is a nop if we were just loaded into memory
    return 0;
}

//
// The following functions expose functionality provided by the Host.
//


/// <summary>
/// printOutput exposes functionaility to print a message to the console or a stringwriter via the host
/// the function is handled in the host via a specific outb message rather than a host function call.
/// </summary>
/// <param name="message">The message to be printed.</param>
/// <returns>The length of the message printed.</returns>

int printOutput(const char* message)
{
    size_t result = strlen(message);
    if (result >= pPeb->outputdata.outputDataSize)
    {
        result = (size_t)pPeb->outputdata.outputDataSize - 1;
    }
    memset(pPeb->outputdata.outputDataBuffer, 0, pPeb->outputdata.outputDataSize);

#pragma warning(suppress : 4996)
    strncpy((char*)pPeb->outputdata.outputDataBuffer, (char*)message, result);
    outb(OUTB_WRITE_OUTPUT, 0);
    return (int)result;
}


// The following host functions are defined in the Sandbox Host in Core/HyperLightExports.cs

/// <summary>
/// This function is required by dlmalloc
/// </summary>
long long GetHyperLightTickCount()
{
    return native_symbol_thunk_returning_longlong("GetTickCount");
}

/// <summary>
/// This function is required/called by WAMR function os_thread_get_stack_boundary() 
/// which is needed for AOT WASM Module execution
/// </summary>
uint8_t* GetStackBoundary()
{
    static unsigned __int64 thread_stack_boundary = 0;
    // If we are not running in Hyperlight then we need to get this information in the host
    if (!runningInHyperlight)
    {
        thread_stack_boundary = native_symbol_thunk_returning_ulonglong("GetStackBoundary");
    }
    else
    {
        if (0 == thread_stack_boundary)
        {
            thread_stack_boundary = pPeb->gueststackData.minStackAddress;
        }
    }
    return (uint8_t*)thread_stack_boundary;
}

/// <summary>
/// This function is required by dlmalloc
/// </summary>
unsigned int GetOSPageSize()
{
    return native_symbol_thunk_returning_uint("GetOSPageSize");
}

/// <summary>
/// This is required by os_time_get_boot_microsecond() in WAMR 
/// </summary>
long long GetTimeSinceBootMicrosecond()
{
    return native_symbol_thunk_returning_longlong("GetTimeSinceBootMicrosecond");
}
