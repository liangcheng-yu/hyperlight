#pragma once

// https://developercommunity.visualstudio.com/t/need-example-of-how-to-use-crt-secure-no-warnings/720988
#define _CRT_SECURE_NO_WARNINGS

#include "munit/munit.h"

MunitResult test_pe_get_headers();
MunitResult test_pe_relocate();

static const size_t NUM_PE_FILES = 2;
static const char *pe_filenames[] = {"./testdata/simpleguest.exe", "./testdata/callbackguest.exe"};

static MunitTest pe_file_tests[] = {
    {
        "/test_pe_relocate",    /* name */
        test_pe_relocate,       /* test */
        NULL,                   /* setup */
        NULL,                   /* tear_down */
        MUNIT_TEST_OPTION_NONE, /* options */
        NULL                    /* parameters */
    },
    {"/test_pe_get_headers",
     test_pe_get_headers,
     NULL,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
