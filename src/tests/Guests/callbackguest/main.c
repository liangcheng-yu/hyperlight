#include "hyperlight.h"
#include <string.h>

int sendMessagetoHostMethod(char* methodName, char* guestMessage, const char* message)
{
#pragma warning(suppress:4244)
    char* messageToHost = strncat(guestMessage, message, strlen(message));

    Hyperlight_Generated_HostFunctionDetails_table_t hostfunctionDetails = GetHostFunctionDetails();
    if (NULL == hostfunctionDetails)
    { 
        setError(GUEST_ERROR, "No host functions found");
    }

    ns(HostFunctionDefinition_vec_t) hostFunctionDefinitions = ns(HostFunctionDetails_functions(hostfunctionDetails));

    size_t key = ns(HostFunctionDefinition_vec_find_by_function_name(hostFunctionDefinitions, methodName));

    if (flatbuffers_not_found == key)
    {
        char message[100];
        snprintf(message, 100, "Host Function Not Found: %s", methodName);
        setError(GUEST_ERROR, message);
    }

    ns(HostFunctionDefinition_table_t) hostFunctionDefiniton = ns(HostFunctionDefinition_vec_at(hostFunctionDefinitions, key));

    Hyperlight_Generated_ReturnType_enum_t returnValueType = Hyperlight_Generated_HostFunctionDefinition_return_type(hostFunctionDefiniton);

    if (returnValueType != Hyperlight_Generated_ReturnType_hlint)
    {
        char message[100];
        snprintf(message, 100, "Host Function  %s has unexpected return type %d", methodName, returnValueType);
        setError(GUEST_ERROR, message);
    }

    Hyperlight_Generated_ParameterType_vec_t parameterTypes =  Hyperlight_Generated_HostFunctionDefinition_parameters(hostFunctionDefiniton);
    size_t numParams = Hyperlight_Generated_ParameterType_vec_len(parameterTypes);
    
    if (numParams != 1)
    {
        char message[100];
        snprintf(message, 100, "Host Function  %s has unexpected number of parameters %d", methodName, numParams);
        setError(GUEST_ERROR, message);
    }

    Hyperlight_Generated_ParameterType_enum_t paramType = Hyperlight_Generated_ParameterType_vec_at(parameterTypes,0);

    if (paramType != Hyperlight_Generated_ParameterType_hlstring)
    {
        char message[100];
        snprintf(message, 100, "Host Function  %s has unexpected parameter type %d", methodName, paramType);
        setError(GUEST_ERROR, message);
    }
    
    return native_symbol_thunk_returning_int(methodName, messageToHost);
}

int guestFunction(const char *message)
{ 
    char guestMessage[256] = "Hello from GuestFunction, ";
    return sendMessagetoHostMethod("HostMethod", guestMessage, message);
}

int guestFunction1(const char* message)
{
    char guestMessage[256] = "Hello from GuestFunction1, ";
    return sendMessagetoHostMethod("HostMethod1", guestMessage, message);
}

int guestFunction2(const char* message)
{
    char guestMessage[256] = "Hello from GuestFunction2, ";
    return sendMessagetoHostMethod("HostMethod1", guestMessage, message);
}

int guestFunction3(const char* message)
{
    char guestMessage[256] = "Hello from GuestFunction3, ";
    return sendMessagetoHostMethod("HostMethod1", guestMessage, message);
}
// TODO: support void return 
int logMessage(const char* message, const char* source, int logLevel)
{
    if (logLevel < 0 || logLevel > 6)
    {
        logLevel = 0;
    }
    LOG((LogLevel)logLevel, message, source);
    return (int)strlen(message);
}

int callErrorMethod(const char* message)
{
    char guestMessage[256] = "Error From Host: ";
    return sendMessagetoHostMethod("ErrorMethod", guestMessage, message);
}

GENERATE_FUNCTION(printOutput, 1, hlstring);
GENERATE_FUNCTION(guestFunction, 1, hlstring);
GENERATE_FUNCTION(guestFunction1, 1, hlstring);
GENERATE_FUNCTION(guestFunction2, 1, hlstring);
GENERATE_FUNCTION(guestFunction3, 1, hlstring);
GENERATE_FUNCTION(logMessage, 3, hlstring, hlstring, hlint);
GENERATE_FUNCTION(callErrorMethod, 1, hlstring);

void HyperlightMain()
{
    RegisterFunction(FUNCTIONDETAILS("PrintOutput", printOutput));
    RegisterFunction(FUNCTIONDETAILS("GuestMethod", guestFunction));
    RegisterFunction(FUNCTIONDETAILS("GuestMethod1", guestFunction1));
    RegisterFunction(FUNCTIONDETAILS("GuestMethod2", guestFunction2));
    RegisterFunction(FUNCTIONDETAILS("GuestMethod3", guestFunction3));
    RegisterFunction(FUNCTIONDETAILS("LogMessage", logMessage));
    RegisterFunction(FUNCTIONDETAILS("CallErrorMethod", callErrorMethod));
}
