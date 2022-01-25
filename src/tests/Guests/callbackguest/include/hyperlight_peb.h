#include <stdint.h>
typedef struct
{
    uint64_t CountOfFunctions;
    uint64_t DispatchFunction;

} PEBHeader;

typedef struct
{
    char* FunctionName;
    char* FunctionSignature;
    uint64_t Flags;
} FunctionDefinition;

typedef struct
{
    PEBHeader header;
} HyperlightPEB;