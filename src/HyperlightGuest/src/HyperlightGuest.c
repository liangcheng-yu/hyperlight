
#include "hyperlight.h"
#include "hyperlight_guest.h"
#include "hyperlight_error.h"
#include "hyperlight_peb.h"
#include <setjmp.h>
#include <string.h>

extern int _fltused = 0;
uintptr_t __security_cookie;
jmp_buf jmpbuf;
HyperlightPEB *pPeb;
flatcc_builder_t GuestFunctionBuilder;
ns(GuestFunctionDetails_table_t) GuestFunctions;
unsigned int OSPageSize = 0;

bool runningInHyperlight = true;
// TODO: This function is only used by C# code, once we have a Rust Hyperlight C API we can remove this. 
void (*outb_ptr)(uint16_t port, uint8_t value) = NULL;
// This function calls outb in process , as the function being called is a Rust Object we need both the function pointer 
// and the object pointer to make the call from C.
void (*outb_ptr_with_context)(void *context, uint16_t port, uint8_t value) = NULL;

/// <summary>
/// This function is required by dlmalloc, its used to get a pseudo radom number , its only called once when malloc is initialized.
/// as such we can use the security cookie seed from the peb as a random number.
/// We do this rather than calling GetTickCount() in the host as malloc is used when making a host call so we want to avoid recursion.
/// </summary>
long long GetHyperLightTickCount()
{
    return pPeb->security_cookie_seed;
}

/// <summary>
/// This is required by os_getpagesize() in WAMR (which is needed to handle AOT compiled WASM)
/// its also used  by dlmalloc
/// </summary>
unsigned int GetOSPageSize()
{
    return OSPageSize;
}

ns(GuestFunctionDefinition_ref_t) CreateFunctionDefinition(const char *functionName, guestFunc pFunction, int paramCount, ns(ParameterType_enum_t) parameterKind[])
{
    flatbuffers_string_ref_t name = flatbuffers_string_create_str(&GuestFunctionBuilder, functionName);

    ns(ParameterType_vec_start(&GuestFunctionBuilder));

    for (int i = 0; i < paramCount; i++)
    {
        ns(ParameterType_vec_push(&GuestFunctionBuilder, &parameterKind[i]));
    }

    ns(ParameterType_vec_ref_t) parameters = ns(ParameterType_vec_end(&GuestFunctionBuilder));

    // TODO Set the return type correctly

    ns(ReturnType_enum_t) returnType = ns(ReturnType_hlint);

    return ns(GuestFunctionDefinition_create(&GuestFunctionBuilder, name, parameters, returnType, (int64_t)pFunction));
}

void RegisterFunction(ns(GuestFunctionDefinition_ref_t) functionDefinition)
{
    ns(GuestFunctionDetails_functions_push(&GuestFunctionBuilder, functionDefinition));
}

void InitialiseFunctionTable()
{
    if (flatcc_builder_init(&GuestFunctionBuilder))
    {
        setError(GUEST_ERROR, "Failed to initialize flatcc Guest Function Builder");
    }
    ns(GuestFunctionDetails_start_as_root(&GuestFunctionBuilder));
    ns(GuestFunctionDetails_functions_start(&GuestFunctionBuilder));
}

void FinaliseFunctionTable()
{

    // Finalise the Guest Function Details table and then get a mutable vec of GuestFunctionDefinitions and sort it by name so that
    // the function definitons in the table are in name order - this is required for find to work.

    ns(GuestFunctionDetails_functions_end(&GuestFunctionBuilder));
    ns(GuestFunctionDetails_end_as_root(&GuestFunctionBuilder));
    size_t size;
    void *buffer = flatcc_builder_finalize_buffer(&GuestFunctionBuilder, &size);
    assert(buffer);
    GuestFunctions = ns(GuestFunctionDetails_as_root(buffer));
    assert(GuestFunctions);
    ns(GuestFunctionDefinition_vec_t) guestFunctionDefinitions = ns(GuestFunctionDetails_functions(GuestFunctions));
    assert(guestFunctionDefinitions);
    ns(GuestFunctionDefinition_mutable_vec_t) mutableGuestFunctionDefinitions = (ns(GuestFunctionDefinition_mutable_vec_t))guestFunctionDefinitions;
    ns(GuestFunctionDefinition_vec_sort_by_function_name(mutableGuestFunctionDefinitions));

    // Dont free the buffer here as we are storing the GuestFunctionDetails flatbuffer for the lifetime of the guest. If we free it here then the GuestFunctionDetails flatbuffer is destroyed.
}

void checkForHostError()
{
    size_t size;
    void *buffer = flatbuffers_read_size_prefix(pPeb->pGuestErrorBuffer, &size);
    assert(NULL != buffer);
    // No need to free buffer as its just a pointer to an offset in the message buffer in the PEB
    if (size > 0)
    {
        ns(GuestError_table_t guestError = ns(GuestError_as_root(buffer)));
        if (ns(GuestError_code(guestError) != ns(ErrorCode_NoError)))
        {
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
    else if (NULL != outb_ptr_with_context)
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
        // uint64_t rsi = getrsi();
        // uint64_t rdi = getrdi();
        // TODO: Remove the if and the else branch once we have a Rust Hyperlight C API
        if (pPeb->pOutbContext != NULL)
        {
            outb_ptr_with_context(pPeb->pOutbContext, port, value);
        }
        else
        {
            outb_ptr(port, value);
        }
        // setrsi(rsi);
        // setrdi(rdi);
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

void writeError(uint64_t errorCode, char *message)
{
    // Create a flatbuffer builder object
    flatcc_builder_t builder;

    // Initialize the builder object.

    if (flatcc_builder_init(&builder))
    {
        // do not call setError here, as it will cause an infinite recursion
        abort();
    }

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

    assert(flatcc_builder_copy_buffer(&builder, (void *)pPeb->pGuestErrorBuffer, flatb_size));
    flatcc_builder_clear(&builder);
}

void resetError()
{
    memset(pPeb->pGuestErrorBuffer, 0, pPeb->guestErrorBufferSize);
}

// SetError sets the specified error and message in memory and then halts execution by returning the the point that setjmp was called

void setError(uint64_t errorCode, char *message)
{
    writeError(errorCode, message);
    longjmp(jmpbuf, 1);
}

// Pushes to the shared output buffer
void push_shared_output_data(void *buffer, size_t size)
{
    // get relative offset to next free address
    uint64_t stack_ptr_rel = *((uint64_t*)pPeb->outputdata.outputDataBuffer);
    
    // check if the stack pointer is within the bounds of the buffer. 
    // It can be equal to the size, but never greater
    // It can never be less than 8. An empty buffer's stack pointer is 8
    if (stack_ptr_rel > pPeb->outputdata.outputDataSize || stack_ptr_rel < 8)
    {
        char message[100];
        snprintf(message, 100, "Corrupt OutputDataBuffer pointer: %lu, OutputDataSize: %lu\n", stack_ptr_rel, pPeb->outputdata.outputDataSize);
        setError(GUEST_ERROR, message);
    }

    // check if there is enough space in the buffer
    size_t size_required = size + 8; // the data plus the pointer pointing to the data
    size_t size_available = pPeb->outputdata.outputDataSize - stack_ptr_rel;
    if (size_required > size_available)
    {
        char message[100];	
        snprintf(message, 100, "Not enough space to push data to shared OutputDataBuffer. Required: %d, Available: %d\n", size_required, size_available);
        setError(GUEST_ERROR, message);
    }

    // write the data
    memcpy((char*)pPeb->outputdata.outputDataBuffer + stack_ptr_rel, buffer, size);

    // write the offset to the newly written data, to the top of the stack
    *((uint64_t*)((char*)pPeb->outputdata.outputDataBuffer + stack_ptr_rel + size)) = stack_ptr_rel;

    // update the stack pointer to point to next free address
    *((uint64_t*) pPeb->outputdata.outputDataBuffer) = stack_ptr_rel + size + 8;
}

// Pops the shared input buffer and returns a pointer to the popped off buffer.
// The caller is responsible for freeing the buffer.
// Returns NULL if there was an error. 
void* pop_shared_input_buffer()
{
    // get relative offset to next free address
    uint64_t stack_ptr_rel = *((uint64_t*)pPeb->inputdata.inputDataBuffer);
    
    // check if the stack pointer is within the bounds of the buffer. 
    // It can be equal to the size, but never greater
    // It can never be less than 16 since empty buffer's stack pointer is 8
    // so, the first element occupies at least 8 bytes
    if (stack_ptr_rel > pPeb->inputdata.inputDataSize || stack_ptr_rel < 16)
    {
        char message[100];
        snprintf(message, 100, "Corrupt InputDataBuffer pointer: %lu, InputDataSize: %lu\n", stack_ptr_rel, pPeb->inputdata.inputDataSize);
        setError(GUEST_ERROR, message);
        return NULL;
    }

    // Convert to absolute and go back 8 bytes. This is the pointer to the offset of the element on the top of the stack
    uint64_t* last_element_pointer_offset_abs = (uint64_t*)((char*)pPeb->inputdata.inputDataBuffer + stack_ptr_rel - 8);

    // read to get the actual offset to the last element
    uint64_t last_element_start_rel = *(last_element_pointer_offset_abs);
    // convert to absolute
    void* last_element_offset_abs = (char*)pPeb->inputdata.inputDataBuffer + last_element_start_rel;

    // read the flatbuffer
    size_t size;
    void *temp_buffer = flatbuffers_read_size_prefix(last_element_offset_abs, &size);
    
    if (size <= 0)
    {
        setError(GUEST_ERROR, "Got a 0-size buffer in pop_shared_input_buffer");
        return NULL;
    }

    // copy the buffer, since otherwise it would be overwritten
    // by subsequent pushes to the shared buffer
    void* buffer = malloc(size + 4); // size is excluding the size prefix
    memcpy(buffer, temp_buffer, size + 4);

    // update the stack pointer to point to the element we just popped off, since that is now free
    *((uint64_t*) pPeb->inputdata.inputDataBuffer) = last_element_start_rel;

    // zero out popped off buffer
    uint64_t num_bytes_to_zero = stack_ptr_rel - last_element_start_rel;
    memset(last_element_offset_abs, 0, num_bytes_to_zero);
    return buffer;
}

void ValidateHostFunctionCall(flatcc_builder_t *HostFunctionCallBuilder, int32_t numArgs, char *functionName, va_list ap)
{
    ns(HostFunctionDetails_table_t) hostfunctionDetails = GetHostFunctionDetails();
    if (NULL == hostfunctionDetails)
    {
        setError(GUEST_ERROR, "No host functions found");
    }

    ns(HostFunctionDefinition_vec_t) hostFunctionDefinitions = ns(HostFunctionDetails_functions(hostfunctionDetails));

    size_t key = ns(HostFunctionDefinition_vec_find_by_function_name(hostFunctionDefinitions, functionName));

    if (flatbuffers_not_found == key)
    {
        char message[100];
        snprintf(message, 100, "Host Function Not Found: %s", functionName);
        setError(GUEST_ERROR, message);
    }

    ns(HostFunctionDefinition_table_t) hostFunctionDefiniton = ns(HostFunctionDefinition_vec_at(hostFunctionDefinitions, key));

    ns(ParameterType_vec_t) parameterTypes = ns(HostFunctionDefinition_parameters(hostFunctionDefiniton));
    size_t numParams = ns(ParameterType_vec_len(parameterTypes));

    if (numArgs != numParams)
    {
        char message[100];
        snprintf(message, 100, "Incorrect number of arguments for host function: %s. Got %d Expected %d", functionName, numArgs, numParams);
        setError(GUEST_ERROR, message);
    }

    ns(Parameter_vec_start(HostFunctionCallBuilder));

    for (int i = 0; i < numParams; i++)
    {
        ns(ParameterType_enum_t) paramType = ns(ParameterType_vec_at(parameterTypes, i));

        switch (paramType)
        {
        case ns(ParameterType_hlint):
        {
            int32_t value = va_arg(ap, int32_t);
            ns(hlint_ref_t) val = ns(hlint_create(HostFunctionCallBuilder, value));
            ns(ParameterValue_union_ref_t) pValue = ns(ParameterValue_as_hlint(val));
            ns(Parameter_ref_t) param = ns(Parameter_create(HostFunctionCallBuilder, pValue));
            ns(FunctionCall_vec_push(HostFunctionCallBuilder, param));
            break; 
        }
        case ns(ParameterType_hllong):
        {
            int64_t value= va_arg(ap, int64_t);
            ns(hllong_ref_t) val = ns(hllong_create(HostFunctionCallBuilder, value));
            ns(ParameterValue_union_ref_t) pValue = ns(ParameterValue_as_hllong(val));
            ns(Parameter_ref_t) param = ns(Parameter_create(HostFunctionCallBuilder, pValue));
            ns(FunctionCall_vec_push(HostFunctionCallBuilder, param));
            break;         
        }
        case ns(ParameterType_hlstring):
        {
            char *value = va_arg(ap, char*);
            flatbuffers_string_ref_t fb_string_ref = flatbuffers_string_create_str(HostFunctionCallBuilder, value);
            ns(hlstring_ref_t) val = ns(hlstring_create(HostFunctionCallBuilder, fb_string_ref));
            ns(ParameterValue_union_ref_t) pValue = ns(ParameterValue_as_hlstring(val));
            ns(Parameter_ref_t) param = ns(Parameter_create(HostFunctionCallBuilder, pValue));
            ns(FunctionCall_vec_push(HostFunctionCallBuilder, param));
            break;     
        }
        case ns(ParameterType_hlbool):
        {
            bool value = va_arg(ap, bool);
            ns(hlbool_ref_t) val = ns(hlbool_create(HostFunctionCallBuilder, value));
            ns(ParameterValue_union_ref_t) pValue = ns(ParameterValue_as_hlbool(val));
            ns(Parameter_ref_t) param = ns(Parameter_create(HostFunctionCallBuilder, pValue));
            ns(FunctionCall_vec_push(HostFunctionCallBuilder, param));
            break;
        }
        case ns(ParameterType_hlvecbytes):
        {
            void* value = va_arg(ap, void*);
            
            // If the parameter is of type then the following parameter must be its length

            ns(ParameterType_enum_t) lenParamType = ns(ParameterType_vec_at(parameterTypes, i++));
            if (lenParamType != ns(ParameterType_hlint))
            {
                char message[100];
                snprintf(message, 100, "Host Function %s: Parameter %d should be length of buffer for parameter %d", functionName, i, i - 1);
                setError(GUEST_ERROR, message);
            }
            int32_t length = 0;
            ns(Parameter_ref_t) lenParam = (int32_t)0;
            if ((length = va_arg(ap, int32_t)) && length > 0)
            {
                ns(hlint_ref_t) val = ns(hlint_create(HostFunctionCallBuilder, length));
                ns(ParameterValue_union_ref_t) pValue = ns(ParameterValue_as_hlint(val));
                lenParam = ns(Parameter_create(HostFunctionCallBuilder, pValue));
            }
            else
            {
                char message[100];
                snprintf(message, 100, "Failed to get int32 parameter: %d for host function: %s", i, functionName);
                setError(GUEST_ERROR, message);
            }
            flatbuffers_uint8_vec_ref_t fb_vec_ref = flatbuffers_uint8_vec_create(HostFunctionCallBuilder, value, length);
            ns(hlvecbytes_ref_t) val = ns(hlvecbytes_create(HostFunctionCallBuilder, fb_vec_ref));
            ns(ParameterValue_union_ref_t) pValue = ns(ParameterValue_as_hlvecbytes(val));
            ns(Parameter_ref_t) param = ns(Parameter_create(HostFunctionCallBuilder, pValue));
            ns(FunctionCall_vec_push(HostFunctionCallBuilder, param));
            ns(FunctionCall_vec_push(HostFunctionCallBuilder, lenParam));
            break;
          
        }
        default:
        {
            char message[100];
            snprintf(message, 100, "Unexpected parameter type: %d for host function: %s", paramType, functionName);
            setError(GUEST_ERROR, message);
        }
        }
    }
    ns(Parameter_vec_ref_t) params_vec = ns(Parameter_vec_end(HostFunctionCallBuilder));
    ns(FunctionCall_parameters_add(HostFunctionCallBuilder, params_vec));
}

void CallHostFunction(char *functionName, int32_t numArgs, va_list ap)
{
    flatcc_builder_t hostFunctionCallBuilder;
    if (flatcc_builder_init(&hostFunctionCallBuilder))
    {
        setError(GUEST_ERROR, "Failed to initialize flatcc Host Function Call builder");
    }

    ns(FunctionCall_start_as_root_with_size(&hostFunctionCallBuilder));
    ns(FunctionCall_function_name_create(&hostFunctionCallBuilder, functionName, strlen(functionName)));
    ns(FunctionCall_function_call_type_add(&hostFunctionCallBuilder, ns(FunctionCallType_host)));

    ValidateHostFunctionCall(&hostFunctionCallBuilder, numArgs, functionName, ap);

    ns(FunctionCall_end_as_root(&hostFunctionCallBuilder));

    size_t size;
    void *buffer = flatcc_builder_finalize_buffer(&hostFunctionCallBuilder, &size);
    assert(buffer);

    push_shared_output_data(buffer, size);
    
    free(buffer);
    flatcc_builder_clear(&hostFunctionCallBuilder);
    outb(OUTB_CALL_FUNCTION, 0);
}

// TODO: Make these functions generic.

// Calls a Host Function that returns an int
int native_symbol_thunk_returning_int(char *functionName,int32_t numArgs, ...)
{

    va_list ap = NULL;

    va_start(ap, numArgs);

    CallHostFunction(functionName, numArgs, ap);

    va_end(ap);

    return GetHostReturnValueAsInt();
}

int GetHostReturnValueAsInt()
{
    void *buffer = pop_shared_input_buffer();
    if (NULL == buffer)
    {
        return -1;
    }

    ns(FunctionCallResult_table_t) funcCallRes = ns(FunctionCallResult_as_root(
        buffer));

    ns(ReturnType_enum_t) retValType = ns(FunctionCallResult_return_value_type(
        funcCallRes));

    if (ns(ReturnValue_hlint) == retValType)
    {
        ns(hlint_table_t) hlintTable = (ns(hlint_table_t))ns(FunctionCallResult_return_value(
            funcCallRes));
        int32_t hlintVal = ns(hlint_value_get(hlintTable));
        free(buffer);
        return hlintVal;
    }
    else
    {
        free(buffer);
        setError(GUEST_ERROR, "Host return value was not an int as expected");
        return -1;
    }
}

// Calls a Host Function that returns void
void native_symbol_thunk(char *functionName,int32_t numArgs, ...)
{

    va_list ap = NULL;

    va_start(ap, numArgs);

    CallHostFunction(functionName, numArgs, ap);

    va_end(ap);
}

// Calls a Host Function that returns an int
unsigned int native_symbol_thunk_returning_uint(char *functionName, int32_t numArgs, ...)
{

    va_list ap = NULL;

    va_start(ap, numArgs);

    CallHostFunction(functionName, numArgs, ap);

    va_end(ap);

    return GetHostReturnValueAsUInt();
}

unsigned int GetHostReturnValueAsUInt()
{
    int intVal = GetHostReturnValueAsInt();
    if (intVal < 0)
    {
        setError(GUEST_ERROR, "Host return value was not a uint as expected");
        return 0;
    }
    else
    {
        return intVal;
    }
}

// Calls a Host Function that returns an long long
long long native_symbol_thunk_returning_longlong(char *functionName, int32_t numArgs, ...)
{

    va_list ap = NULL;

    va_start(ap, numArgs);

    CallHostFunction(functionName, numArgs, ap);

    va_end(ap);

    return GetHostReturnValueAsLongLong();
}

long long GetHostReturnValueAsLongLong()
{
    void *buffer = pop_shared_input_buffer();
    if (NULL == buffer)
    {
        return -1;
    }

    ns(FunctionCallResult_table_t) funcCallRes = ns(FunctionCallResult_as_root(
        buffer));

    ns(ReturnType_enum_t) retValType = ns(FunctionCallResult_return_value_type(
        funcCallRes));

    if (ns(ReturnValue_hllong) == retValType)
    {
        ns(hllong_table_t) hllongTable = (ns(hllong_table_t))ns(FunctionCallResult_return_value(
            funcCallRes));
        int64_t hllongVal = ns(hllong_value_get(hllongTable));
        free(buffer);
        return hllongVal;
    }
    else
    {
        free(buffer);
        setError(GUEST_ERROR, "Host return value was not an int as expected");
        return -1;
    }
}

// Calls a Host Function that returns an ulong long
unsigned long long native_symbol_thunk_returning_ulonglong(char *functionName, int32_t numArgs, ...)
{

    va_list ap = NULL;

    va_start(ap, numArgs);

    CallHostFunction(functionName, numArgs, ap);

    va_end(ap);

    return GetHostReturnValueAsULongLong();
}

unsigned long long GetHostReturnValueAsULongLong()
{
    long long longLongRet = GetHostReturnValueAsLongLong();
    if (longLongRet < 0)
    {
        setError(GUEST_ERROR, "Host return value was not a ulonglong as expected");
        return 0;
    }
    else
    {
        return longLongRet;
    }
}

void GetFunctionCallParameters(ns(FunctionCall_table_t) functionCall, Parameter parameterValues[])
{
    ns(Parameter_vec_t) parameters = ns(FunctionCall_parameters(functionCall));
    size_t actualParameterCount = ns(Parameter_vec_len(parameters));

    for (int i = 0; i < actualParameterCount; i++)
    {
        ns(Parameter_table_t) parameter = ns(Parameter_vec_at(parameters, i));
        ns(ParameterValue_union_type_t) parameterType = ns(Parameter_value_type(parameter));

        switch (parameterType)
        {
        case ns(ParameterValue_hlint):
            parameterValues[i].kind = hlint;
            ns(hlint_table_t) hlintTable = ns(Parameter_value(parameter));
            parameterValues[i].value.hlint = ns(hlint_value(hlintTable));
            break;
        case ns(ParameterValue_hllong):
            parameterValues[i].kind = hllong;
            ns(hllong_table_t) hllongTable = ns(Parameter_value(parameter));
            parameterValues[i].value.hllong = ns(hllong_value(hllongTable));
            break;
        case ns(ParameterValue_hlstring):
            parameterValues[i].kind = hlstring;
            ns(hlstring_table_t) hlstringTable = ns(Parameter_value(parameter));
            parameterValues[i].value.hlstring = ns(hlstring_value(hlstringTable));
            break;
        case ns(ParameterValue_hlbool):
            parameterValues[i].kind = hlbool;
            ns(hlbool_table_t) hlboolTable = ns(Parameter_value(parameter));
            parameterValues[i].value.hlbool = ns(hlbool_value(hlboolTable));
            break;
        case ns(ParameterValue_hlvecbytes):
            parameterValues[i].kind = hlvecbytes;
            ns(hlvecbytes_table_t) hlvecbytesTable = ns(Parameter_value(parameter));
            parameterValues[i].value.hlvecbytes = ns(hlvecbytes_value(hlvecbytesTable));
            break;
        default:
            // This should be impossible as we have validated that the correct parameter types are present.
            break;
        }
    }
}

uint8_t *GetFlatBufferResult(flatbuffers_builder_t *functionCallResultBuilder, ns(ReturnValue_union_ref_t) returnValue)
{
    ns(FunctionCallResult_start_as_root_with_size(functionCallResultBuilder));
    ns(FunctionCallResult_return_value_add(functionCallResultBuilder, returnValue));
    ns(FunctionCallResult_end_as_root(functionCallResultBuilder));
    uint8_t *buffer;
    size_t size;
    buffer = (uint8_t *)flatcc_builder_finalize_buffer(functionCallResultBuilder, &size);
    flatcc_builder_clear(functionCallResultBuilder);
    return buffer;
}

uint8_t *GetFlatBufferResultFromInt(uint32_t value)
{
    flatbuffers_builder_t functionCallResultBuilder;
    if (flatcc_builder_init(&functionCallResultBuilder))
    {
        setError(GUEST_ERROR, "Failed to initialize flatcc Function Call Result Builder");
    }
    ns(hlint_ref_t) hlintVal = ns(hlint_create(&functionCallResultBuilder, value));
    ns(ReturnValue_union_ref_t) returnValue = ns(ReturnValue_as_hlint(hlintVal));
    return GetFlatBufferResult(&functionCallResultBuilder, returnValue);
}

uint8_t *GetFlatBufferResultFromVoid()
{
    flatbuffers_builder_t functionCallResultBuilder;
    if (flatcc_builder_init(&functionCallResultBuilder))
    {
        setError(GUEST_ERROR, "Failed to initialize flatcc Function Call Result Builder");
    }
    ns(hlvoid_ref_t) hlvoidVal = ns(hlvoid_create(&functionCallResultBuilder));
    ns(ReturnValue_union_ref_t) returnValue = ns(ReturnValue_as_hlvoid(hlvoidVal));
    return GetFlatBufferResult(&functionCallResultBuilder, returnValue);
}

uint8_t *GetFlatBufferResultFromString(const char *value)
{
    flatbuffers_builder_t functionCallResultBuilder;
    if (flatcc_builder_init(&functionCallResultBuilder))
    {
        setError(GUEST_ERROR, "Failed to initialize flatcc Function Call Result Builder");
    }
    flatbuffers_string_ref_t fbstringVal = flatbuffers_string_create_str(&functionCallResultBuilder, value);
    ns(hlstring_ref_t) hlstringVal = ns(hlstring_create(&functionCallResultBuilder, fbstringVal));
    ns(ReturnValue_union_ref_t) returnValue = ns(ReturnValue_as_hlstring(hlstringVal));
    return GetFlatBufferResult(&functionCallResultBuilder, returnValue);
}

uint8_t *GetFlatBufferResultFromSizePrefixedBuffer(void *value, int32_t length)
{
    flatbuffers_builder_t functionCallResultBuilder;
    if (flatcc_builder_init(&functionCallResultBuilder))
    {
        setError(GUEST_ERROR, "Failed to initialize flatcc Function Call Result Builder");
    }
    flatbuffers_uint8_vec_ref_t vecValue = flatbuffers_uint8_vec_create(&functionCallResultBuilder, value, length);
    ns(hlsizeprefixedbuffer_ref_t) hlsizeprefixedbufferVal = ns(hlsizeprefixedbuffer_create(&functionCallResultBuilder, length, vecValue));
    ns(ReturnValue_union_ref_t) returnValue = ns(ReturnValue_as_hlsizeprefixedbuffer(hlsizeprefixedbufferVal));
    return GetFlatBufferResult(&functionCallResultBuilder, returnValue);
}

uint8_t *CallGuestFunction(ns(FunctionCall_table_t) functionCall)
{
    guestFunc pFunction = NULL;
    ns(Parameter_vec_t) parameters = ns(FunctionCall_parameters(functionCall));
    size_t actualParameterCount = ns(Parameter_vec_len(parameters));
    flatbuffers_string_t functionName = ns(FunctionCall_function_name(functionCall));

    // this should not be able to happen

    if (NULL == functionName)
    {
        setError(GUEST_FUNCTION_NAME_NOT_PROVIDED, NULL);
    }

    ns(GuestFunctionDefinition_vec_t) guestFunctionDefinitions = ns(GuestFunctionDetails_functions(GuestFunctions));

    size_t key = ns(GuestFunctionDefinition_vec_find_by_function_name(guestFunctionDefinitions, functionName));

    if (flatbuffers_not_found == key)
    {
        // If the function was not found call the GuestDispatchFunction method optionally provided by the guest program.
        // This allows functions exposed by a guest runtime (e.g. WASM) to be called.
        return GuestDispatchFunction(functionCall);
    }

    ns(GuestFunctionDefinition_table_t) functionDefiniton = ns(GuestFunctionDefinition_vec_at(guestFunctionDefinitions, key));

    pFunction = (guestFunc)ns(GuestFunctionDefinition_function_pointer(functionDefiniton));
    ns(ParameterType_vec_t) parameterTypes = ns(GuestFunctionDefinition_parameters(functionDefiniton));
    size_t requiredParameterCount = ns(ParameterType_vec_len(parameterTypes));

    if (requiredParameterCount != actualParameterCount)
    {
        char message[100] = {0};
        snprintf(message, 100, "Called function %s with %d parameters but it takes %d.", functionName, actualParameterCount, requiredParameterCount);
        setError(GUEST_FUNCTION_INCORRECT_NO_OF_PARAMETERS, message);
    }

    // Get the parameter types from the function call so we can check that the types align and also check for length parsmetet when parameter type is ParameterType_hlvecbytes.
    // The latter check really shouldnt be in the runtime but we dont have anywhere else to put it at the moment.

    ns(ParameterType_enum_t *) parameterKind = (ns(ParameterType_enum_t *))calloc(requiredParameterCount, sizeof(ns(ParameterType_enum_t)));
    bool nextParamIsLength = false;

    for (int i = 0; i < requiredParameterCount; i++)
    {
        ns(Parameter_table_t) parameter = ns(Parameter_vec_at(parameters, i));
        ns(ParameterValue_union_type_t) parameterType = ns(Parameter_value_type(parameter));
        if (nextParamIsLength)
        {
            // The parameter should be a length parameter
            if (parameterType != ns(ParameterValue_hlint))
            {
                char message[15];
                snprintf(message, 15, "Parameter %d", i);
                setError(ARRAY_LENGTH_PARAM_IS_MISSING, message);
            }
            nextParamIsLength = false;
        }

        switch (parameterType)
        {
        case ns(ParameterValue_hlint):
            parameterKind[i] = ns(ParameterType_hlint);
            break;
        case ns(ParameterValue_hllong):
            parameterKind[i] = ns(ParameterType_hllong);
            break;
        case ns(ParameterValue_hlstring):
            parameterKind[i] = ns(ParameterType_hlstring);
            break;
        case ns(ParameterValue_hlbool):
            parameterKind[i] = ns(ParameterType_hlbool);
            break;
        case ns(ParameterValue_hlvecbytes):
            parameterKind[i] = ns(ParameterType_hlvecbytes);
            // Its required that the next parameter is the length of the array.
            nextParamIsLength = true;
            break;
        default:
            // This should not happen unless additional types are added to the schema and guest code is not udpated to take account of it.
            char message[100] = {0};
            snprintf(message, 100, "Unexpected Parameter Type %d in Function %s", parameterType, functionName);
            setError(UNSUPPORTED_PARAMETER_TYPE, message);
        }
    }

    // If the last parameter was a  ParameterValue_hlvecbytes then it should have been followed by a ParameterValue_hlint

    if (nextParamIsLength)
    {
        setError(ARRAY_LENGTH_PARAM_IS_MISSING, "Last parameter should be the length of the array");
    }

    // Check that the parameter types match the expected types.

    for (int i = 0; i < requiredParameterCount; i++)
    {
        if (parameterKind[i] != ns(ParameterType_vec_at(parameterTypes, i)))
        {
            char message[100] = {0};
            snprintf(message, 100, "Function %s parameter %d.", functionName, i);
            setError(GUEST_FUNCTION_PARAMETER_TYPE_MISMATCH, message);
        }
    }
    free(parameterKind);

    // call the function

    return pFunction(functionCall);
}

void DispatchFunction()
{
    // setjmp is used to capture state so that if an error occurs then lngjmp is called in setError and control returns to this point , the if returns false and the program exits/halts
    void* buffer = NULL;
    if (!setjmp(jmpbuf))
    {
        resetError();

        size_t size;

        // pop the shared input buffer
        buffer = pop_shared_input_buffer();
        assert(NULL != buffer);

        ns(FunctionCall_table_t functionCall = ns(FunctionCall_as_root(buffer)));
        // Validate this is a Guest Function Call
        if (ns(FunctionCall_function_call_type(functionCall)) != ns(FunctionCallType_guest))
        {
            setError(GUEST_ERROR, "Invalid Function Call Type");
        }

        // dispatch the function and get result as a flatbuffer
        uint8_t *result = CallGuestFunction(functionCall);

        // Confirm the result is a flatbuffer
        assert(flatbuffers_read_size_prefix(result, &size));

        // Push result
        push_shared_output_data(result, size + 4);
        free(result);
    }

    if (buffer != NULL)
    {
        free(buffer);
    }
    halt(); // This is a nop if we were just loaded into memory
}

// this is required by print functions to write output.
// calls from print functions are buffered until
// either the output buffer is full or a null character is sent

void _putchar(char c)
{
    static int index = 0;
    static char message[1000] = {0};

    if (index == 999)
    {
        message[index] = '\0';
        printOutput(message);
        index = 0;
        memset(message, 0, 1000);
    }

    if (c != '\0')
    {
        message[index++] = c;
        return;
    }

    message[index++] = c;
    message[index] = '\0';

    printOutput(message);
    index = 0;
    memset(message, 0, 1000);
}

void WriteLogData(
    LogLevel logLevel,
    const char *message,
    const char *source,
    const char *caller,
    const char *sourceFile,
    int32_t line)
{
    flatbuffers_builder_t logDataBuilder;
    if (flatcc_builder_init(&logDataBuilder))
    {
        setError(
            GUEST_ERROR,
            "Failed to initialize flatcc log data builder");
    }

    Hyperlight_Generated_GuestLogData_start_as_root_with_size(&logDataBuilder);

    {
        // write message
        Hyperlight_Generated_GuestLogData_message_create(
            &logDataBuilder,
            message,
            strlen(message));
    }
    {
        // write log level
        Hyperlight_Generated_LogLevel_enum_t level_fb;
        if (logLevel == TRACE)
        {
            level_fb = Hyperlight_Generated_LogLevel_Trace;
        }
        else if (logLevel == DEBUG)
        {
            level_fb = Hyperlight_Generated_LogLevel_Debug;
        }
        else if (logLevel == INFORMATION)
        {
            level_fb = Hyperlight_Generated_LogLevel_Information;
        }
        else if (logLevel == WARNING)
        {
            level_fb = Hyperlight_Generated_LogLevel_Warning;
        }
        else if (logLevel == ERROR)
        {
            level_fb = Hyperlight_Generated_LogLevel_Error;
        }
        else if (logLevel == CRTICAL)
        {
            level_fb = Hyperlight_Generated_LogLevel_Critical;
        }
        else
        {
            level_fb = Hyperlight_Generated_LogLevel_None;
        }
        Hyperlight_Generated_GuestLogData_level_add(
            &logDataBuilder,
            level_fb);
    }
    {
        // write source
        Hyperlight_Generated_GuestLogData_source_create(
            &logDataBuilder,
            source,
            strlen(source));
    }
    {
        // write caller
        Hyperlight_Generated_GuestLogData_caller_create(
            &logDataBuilder,
            caller,
            strlen(caller));
    }
    {
        // write source file
        Hyperlight_Generated_GuestLogData_source_file_create(
            &logDataBuilder,
            sourceFile,
            strlen(sourceFile));
    }
    {
        // write line
        Hyperlight_Generated_GuestLogData_line_add(
            &logDataBuilder,
            line);
    }

    Hyperlight_Generated_GuestLogData_end_as_root(&logDataBuilder);
    size_t size;
    flatcc_builder_finalize_buffer(&logDataBuilder, &size);
    void *buffer = flatcc_builder_get_direct_buffer(&logDataBuilder, &size);
    {
        push_shared_output_data(buffer, size);
    }
    flatcc_builder_clear(&logDataBuilder);
    free(buffer);
}

void Log(LogLevel logLevel, const char *message, const char *source, const char *caller, const char *sourceFile, int32_t line)
{
    WriteLogData(logLevel, message, source, caller, sourceFile, line);
    outb(OUTB_LOG, 0);
}

// this is called when /Gs check fails

void report_gsfailure()
{
    setError(GS_CHECK_FAILED, NULL);
}

void __report_rangecheckfailure(void)
{
    report_gsfailure();
}

// Called by dlmalloc using ABORT

// TODO: Once we update the host to deal with aborts correctly we should update these handlers so that they return a code to explain what caused the abort
// i.e. they should be updated to call abort_with_code(ERROR_CODE) instead of abort()

void abort_with_code_and_message(uint32_t code, char* message)
{
    void* guestPanicDataBuffer = pPeb->GuestPanicContextData.guestPanicContextDataBuffer;
    size_t guestPanicDataBufferSize = pPeb->GuestPanicContextData.guestPanicContextDataSize;

    memcpy(
        guestPanicDataBuffer,
        message,
        guestPanicDataBufferSize);

    outb(OUTB_ABORT, code);
}

void dlmalloc_abort()
{
    WriteLogData(CRTICAL, "dlmalloc_abort", "HyperLightGuest", __FUNCTION__, __FILE__, __LINE__);
    size_t message_size = 256;
    char message[256] = {'0'};
    snprintf(message, message_size, "dlmalloc_abort in HyperlightGuest");
    abort_with_code_and_message(FAILURE_IN_DLMALLOC, message);
}

void __assert_fail(const char *expr, const char *file, int line, const char *func)
{
    size_t message_size = 256;
    char message[256] = {'0'};
    snprintf(message, message_size, "Assertion failed: %s ", expr);
    WriteLogData(CRTICAL, message, "HyperLightGuest", func, file, line);
    abort_with_code_and_message(UNKNOWN_ERROR, message);
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

void *hyperlightMoreCore(size_t size)
{
    void *ptr = NULL;
    static void *unusedHeapBufferPointer = 0;
    static size_t allocated = 0;
    if (size > 0)
    {
        // Trying to use more memory than is available.
        // In theory this should not happen if dlmalloc_set_footprint_limit was called with pPeb->guestheapData.guestHeapSize.
        // but it appears that it does , we should not setError here as that can cause an infinite loop.
        if (allocated + size > pPeb->guestheapData.guestHeapSize)
        {
            //TODO: Add data to the abort so we know what happened in the host.
            abort();
        }

        if (0 == unusedHeapBufferPointer)
        {
            ptr = (char *)pPeb->guestheapData.guestHeapBuffer;
        }
        else
        {
            ptr = (char *)unusedHeapBufferPointer;
        }

        allocated += size;
        unusedHeapBufferPointer = (char *)ptr + size;
        return ptr;
    }
    else if (size < 0)
    {
        // This should not happen according to dlmalloc docs as MORECORE_CANNOT_TRIM is set.
        char message[100] = {0};
        snprintf(message, 100, "HyperlightMoreCore Unexpected Error trim called with size: %d", size);
        setError(FAILURE_IN_DLMALLOC, message);
    }
    else
    {
        return unusedHeapBufferPointer;
    }
}

ns(HostFunctionDetails_table_t) GetHostFunctionDetails()
{

    // read the Host Fuction Details flatbuffer from memory

    size_t size;
    void *buffer = flatbuffers_read_size_prefix(pPeb->hostFunctionDefinitions.fbHostFunctionDetails, &size);
    assert(NULL != buffer);
    ns(HostFunctionDetails_table_t) hostFunctionDetails = ns(HostFunctionDetails_as_root(buffer));

    return hostFunctionDetails;
}

// maxLogLevel is the maximum log level that the guest can log at, its not used in the C guest
__declspec(safebuffers) int entryPoint(uint64_t pebAddress, uint64_t seed, int osPageSize, int maxLogLevel)
{

    // TODO: We should try and write to stderr here in case the program was started from the command line, note that we dont link against the CRT so we cant use printf
    pPeb = (HyperlightPEB *)pebAddress;
    if (NULL == pPeb)
    {
        return -1;
    }
    __security_init_cookie();

    // setjmp is used to capture state so that if an error occurs then lngjmp is called in setError and control returns to this point , the if returns false and the program exits/halts
    if (!setjmp(jmpbuf))
    {
        OSPageSize = (unsigned int)osPageSize;
        // Either in WHP partition (hyperlight) or in memory.  If in memory, outb_ptr will be non-NULL
        // TODO: Remove the following line once we have a full Rust based C API for Hyperlight.
        outb_ptr = (void (*)(uint16_t, uint8_t))pPeb->pOutb;
        outb_ptr_with_context = (void (*)(void *, uint16_t, uint8_t))pPeb->pOutb;
        if (outb_ptr_with_context)
            runningInHyperlight = false;

        pPeb->guest_function_dispatch_ptr = (uint64_t)DispatchFunction;

        dlmalloc_set_footprint_limit(pPeb->guestheapData.guestHeapSize);

        resetError();

        InitialiseFunctionTable();

        // Call Hyperlightmain in the Guest , the guest can use this to register functions that can be called from the host.

        HyperlightMain();

        // Once the guest has added all the functions it wants to expose to the host, call this function to finalise the function table.
        // This is done as the we are using flatbuffers to serialise the function table and its easier and faster than trying to mutate the tabel each ime it is updated.

        FinaliseFunctionTable();
    }

    halt(); // This is a nop if we were just loaded into memory
    return 0; // this is never checked
}

/// <summary>
/// printOutputAsGuestFunction exposes functionaility to print a message to the console in the host as a Guest Function.
/// </summary>
/// <param name="message">The message to be printed.</param>
/// <returns>The length of the message printed.</returns>

uint8_t* printOutputAsGuestFunction(const char* message)
{
    return GetFlatBufferResultFromInt(native_symbol_thunk_returning_int("HostPrint", 1, message));
}

//
// The following functions expose functionality provided by the Host.
//

/// <summary>
/// printOutput exposes functionaility to print a message to the console in the host as a Guest Function.
/// </summary>
/// <param name="message">The message to be printed.</param>
/// <returns>The length of the message printed.</returns>

int printOutput(const char* message)
{
    return native_symbol_thunk_returning_int("HostPrint", 1, message);
}


/// <summary>
/// This function is required/called by WAMR function os_thread_get_stack_boundary()
/// which is needed for AOT WASM Module execution
/// </summary>
uint8_t *GetStackBoundary()
{
    static unsigned __int64 thread_stack_boundary = 0;
    // If we are not running in Hyperlight then we need to get this information in the host
    if (!runningInHyperlight)
    {
        thread_stack_boundary = native_symbol_thunk_returning_ulonglong("GetStackBoundary",0);
    }
    else
    {
        if (0 == thread_stack_boundary)
        {
            thread_stack_boundary = pPeb->gueststackData.minStackAddress;
        }
    }
    return (uint8_t *)thread_stack_boundary;
}

/// <summary>
/// This is required by os_time_get_boot_microsecond() in WAMR
/// </summary>
long long GetTimeSinceBootMicrosecond()
{
    return native_symbol_thunk_returning_longlong("GetTimeSinceBootMicrosecond", 0);
}
