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

int sendMessagetoHostMethod(char* methodName, char* guestMessage, char* message)
{
    char* messageToHost = strncat(guestMessage, message, strlen(message));
    return native_symbol_thunk(methodName, messageToHost);
}

int guestFunction(char *message)
{ 
    char guestMessage[256] = "Hello from GuestFunction, ";
    sendMessagetoHostMethod("HostMethod", guestMessage, message);
}

int guestFunction1(char* message)
{
    char guestMessage[256] = "Hello from GuestFunction1, ";
    sendMessagetoHostMethod("HostMethod1", guestMessage, message);
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