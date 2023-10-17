#include "hyperlight.h"
#include <string.h>

uint8_t* sendMessagetoHostMethod(char* methodName, char* guestMessage, const char* message)
{
#pragma warning(suppress:4244)
    char* messageToHost = strncat(guestMessage, message, strlen(message));
    int result = native_symbol_thunk_returning_int(methodName, messageToHost);
    return GetFlatBufferResultFromInt(result);
}

uint8_t* guestFunction(const char *message)
{ 
    char guestMessage[256] = "Hello from GuestFunction, ";
    return sendMessagetoHostMethod("HostMethod", guestMessage, message);
}

uint8_t* guestFunction1(const char* message)
{
    char guestMessage[256] = "Hello from GuestFunction1, ";
    return sendMessagetoHostMethod("HostMethod1", guestMessage, message);
}

uint8_t* guestFunction2(const char* message)
{
    char guestMessage[256] = "Hello from GuestFunction2, ";
    return sendMessagetoHostMethod("HostMethod1", guestMessage, message);
}

uint8_t* guestFunction3(const char* message)
{
    char guestMessage[256] = "Hello from GuestFunction3, ";
    return sendMessagetoHostMethod("HostMethod1", guestMessage, message);
}

uint8_t* guestFunction4()
{
    char guestMessage[256] = "Hello from GuestFunction4";
    native_symbol_thunk("HostMethod4", guestMessage);
    return GetFlatBufferResultFromVoid();
}

// TODO: update to support void return 
uint8_t* logMessage(const char* message, const char* source, int logLevel)
{
    if (logLevel < 0 || logLevel > 6)
    {
        logLevel = 0;
    }
    LOG((LogLevel)logLevel, message, source);
    int result = (int)strlen(message);
    return GetFlatBufferResultFromInt(result);
}

uint8_t* callErrorMethod(const char* message)
{
    char guestMessage[256] = "Error From Host: ";
    return sendMessagetoHostMethod("ErrorMethod", guestMessage, message);
}

// Calls a method n the host that should keep the CPU busy forever

uint8_t* callHostSpin()
{

    native_symbol_thunk("Spin");
    return GetFlatBufferResultFromVoid();
}

GENERATE_FUNCTION(printOutput, 1, hlstring);
GENERATE_FUNCTION(guestFunction, 1, hlstring);
GENERATE_FUNCTION(guestFunction1, 1, hlstring);
GENERATE_FUNCTION(guestFunction2, 1, hlstring);
GENERATE_FUNCTION(guestFunction3, 1, hlstring);
GENERATE_FUNCTION(guestFunction4, 0);
GENERATE_FUNCTION(logMessage, 3, hlstring, hlstring, hlint);
GENERATE_FUNCTION(callErrorMethod, 1, hlstring);
GENERATE_FUNCTION(callHostSpin, 0);

void HyperlightMain()
{
    RegisterFunction(FUNCTIONDETAILS("PrintOutput", printOutput));
    RegisterFunction(FUNCTIONDETAILS("GuestMethod", guestFunction));
    RegisterFunction(FUNCTIONDETAILS("GuestMethod1", guestFunction1));
    RegisterFunction(FUNCTIONDETAILS("GuestMethod2", guestFunction2));
    RegisterFunction(FUNCTIONDETAILS("GuestMethod3", guestFunction3));
    RegisterFunction(FUNCTIONDETAILS("GuestMethod4", guestFunction4));
    RegisterFunction(FUNCTIONDETAILS("LogMessage", logMessage));
    RegisterFunction(FUNCTIONDETAILS("CallErrorMethod", callErrorMethod));
    RegisterFunction(FUNCTIONDETAILS("CallHostSpin", callHostSpin));
}
