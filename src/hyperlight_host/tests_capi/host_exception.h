#pragma once
#include "munit/munit.h"

MunitResult test_has_host_exception();
MunitResult test_host_exception_length();
MunitResult test_long_data_causes_errors();
MunitResult test_host_exception_data_round_trip();

static MunitTest host_exception_tests[] = {
    {
        "/test_has_host_exception", /* name */
        test_has_host_exception,    /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {
        "/test_host_exception_length", /* name */
        test_host_exception_length,    /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {
        "/test_long_data_causes_errors", /* name */
        test_long_data_causes_errors,    /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {
        "/test_host_exception_data_round_trip", /* name */
        test_host_exception_data_round_trip,    /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
