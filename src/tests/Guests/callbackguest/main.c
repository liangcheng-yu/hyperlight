#include "hyperlight.h"

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

int sendMessagetoHostMethod(char* methodName, char* guestMessage, const char* message)
{
#pragma warning(suppress:4244)
    char* messageToHost = strncat(guestMessage, message, strlen(message));

    HostFunctionDetails* hostfunctionDetails = GetHostFunctionDetails();
    if (NULL == hostfunctionDetails)
    { 
        setError(GUEST_ERROR, "No host functions found");
    }

    HostFunctionDefinition* hostFunctionDefinition = NULL;

#pragma warning(suppress:6011)
    for (int i = 0; i < hostfunctionDetails->CountOfFunctions; i++)
    {
        if (strcmp(methodName, hostfunctionDetails->HostFunctionDefinitions[i].FunctionName) == 0)
        {
            hostFunctionDefinition = &hostfunctionDetails->HostFunctionDefinitions[i];
            break;
        }
    }

    if (NULL == hostFunctionDefinition)
    {
        char message[100];
        snprintf(message, 100, "Host Function Not Found: %s", methodName);
        setError(GUEST_ERROR, message);
    }
#pragma warning(suppress:6011)
    if (strcmp(hostFunctionDefinition->FunctionSignature, "($)$i") != 0)
    {
        char message[100];
        snprintf(message, 100, "Host Function  %s has unexpected signature %s", methodName, hostFunctionDefinition->FunctionSignature);
        setError(GUEST_ERROR, message);
    }

    free(hostfunctionDetails);
    return native_symbol_thunk(methodName, messageToHost);
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

GENERATE_FUNCTION(printOutput, 1, string);
GENERATE_FUNCTION(guestFunction, 1, string);
GENERATE_FUNCTION(guestFunction1, 1, string);

void HyperlightMain()
{
    RegisterFunction("PrintOutput", FUNCTIONDETAILS(printOutput));
    RegisterFunction("GuestMethod", FUNCTIONDETAILS(guestFunction));
    RegisterFunction("GuestMethod1", FUNCTIONDETAILS(guestFunction1)); 
}