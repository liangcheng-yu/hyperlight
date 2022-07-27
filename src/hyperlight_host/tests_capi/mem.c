#include "stdlib.h"
#include "stdbool.h"
#include "stdint.h"
#include "mem.h"

int8_t *create_i8_mem(uint8_t len, bool fill)
{
    int8_t *mem = (int8_t *)malloc(len * sizeof(int8_t));
    if (fill)
    {
        for (int8_t i = 0; i < len; ++i)
        {
            mem[i] = i;
        }
    }
    return mem;
}

uint8_t *create_u8_mem(uint8_t len, bool fill)
{
    uint8_t *mem = (uint8_t *)malloc(len * sizeof(uint8_t));
    if (fill)
    {
        for (uint8_t i = 0; i < len; ++i)
        {
            mem[i] = i;
        }
    }
    return mem;
}
