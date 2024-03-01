#pragma once

// https://developercommunity.visualstudio.com/t/need-example-of-how-to-use-crt-secure-no-warnings/720988
#define _CRT_SECURE_NO_WARNINGS

#include "munit/munit.h"

MunitResult test_byte_array_null_ptr(const MunitParameter params[], void *user_data);
MunitResult test_byte_array_lifecycle(const MunitParameter params[], void *user_data);
MunitResult test_byte_array_new_from_file(const MunitParameter params[], void *user_data);
long file_size(const char *fname);

static MunitTest byte_array_tests[] = {
    {
        (char *)"/test_byte_array_null_ptr", /* name */
        test_byte_array_null_ptr,            /* test */
        NULL,                                /* setup */
        NULL,                                /* tear_down */
        MUNIT_TEST_OPTION_NONE,              /* options */
        NULL                                 /* parameters */
    },
    {
        (char *)"/test_byte_array_lifecycle", /* name */
        test_byte_array_lifecycle,            /* test */
        NULL,                                 /* setup */
        NULL,                                 /* tear_down */
        MUNIT_TEST_OPTION_NONE,               /* options */
        NULL                                  /* parameters */
    },
    {(char *)"/test_byte_array_new_from_file",
     test_byte_array_new_from_file,
     NULL,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
