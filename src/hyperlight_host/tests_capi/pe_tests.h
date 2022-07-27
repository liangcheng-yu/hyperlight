#pragma once

// https://developercommunity.visualstudio.com/t/need-example-of-how-to-use-crt-secure-no-warnings/720988
#define _CRT_SECURE_NO_WARNINGS

#include "munit/munit.h"

MunitResult test_pe_getters(void);
MunitResult test_pe_relocate(void);

static const size_t NUM_PE_FILES = 2;
static const char *pe_filenames[] = {"./testdata/simpleguest.exe", "./testdata/callbackguest.exe"};
