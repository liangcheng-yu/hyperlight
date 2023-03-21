#pragma once
#include "munit/munit.h"

MunitResult test_write_guest_function_call();

static MunitTest guest_func_tests[] = {
    {
        "/test_write_guest_function_call", /* name */
        test_write_guest_function_call,    /* test */
        NULL,                          /* setup */
        NULL,                          /* tear_down */
        MUNIT_TEST_OPTION_NONE,        /* options */
        NULL                           /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};