#pragma once

// https://developercommunity.visualstudio.com/t/need-example-of-how-to-use-crt-secure-no-warnings/720988
#define _CRT_SECURE_NO_WARNINGS

#include "munit/munit.h"

MunitResult test_byte_array_lifecycle(void);
MunitResult test_byte_array_new_from_file(void);
long file_size(const char *fname);
