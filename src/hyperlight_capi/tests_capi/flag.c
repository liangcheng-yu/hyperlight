
#include "flag.h"
#include "stdbool.h"
// strings.h contains strcasecmp but is only available on Linux.
// On Windows, use string.h and alias the _stricmp function to strcasecmp.
// Documentation for that function
// https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/stricmp-wcsicmp-mbsicmp-stricmp-l-wcsicmp-l-mbsicmp-l?view=msvc-170
#if defined(__linux__)
#include "strings.h"
#else
#include <string.h>
#define strcasecmp _stricmp
#endif

bool get_flag_value(char *flag_value)
{
    if (strlen(flag_value) == 0)
    {
        return false;
    }

    if (strcasecmp(flag_value, "true") == 0 || strcasecmp(flag_value, "1") == 0)
    {
        return true;
    }
    return false;
}
