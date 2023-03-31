#include "hyperlight.h"
#include <string.h>

int sendMessagetoHostMethod(char* methodName, char* guestMessage, const char* message)
{
#pragma warning(suppress:4244)
    char* messageToHost = strncat(guestMessage, message, strlen(message));
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
