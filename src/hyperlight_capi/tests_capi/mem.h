#pragma once

#include "stdlib.h"
#include "stdint.h"
#include "stdbool.h"

// create an array of `len` int8_t values on the heap and optionally
// fill it with integers.
//
// you must free the returned memory exactly once after
// you're done with it.
//
// Note: if `len` overflows and `fill` is true, the returned
// memory may not be completely initialized.
int8_t *create_i8_mem(uint8_t len, bool fill);

// equivalent to create_i8_mem, except with uint8_t values rather than
// int8_t values.
//
// Note: if `len` overflows and `fill` is true,
// the returned memory may not be completely initialized.
uint8_t *create_u8_mem(uint8_t len, bool fill);
