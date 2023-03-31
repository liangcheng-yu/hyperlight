#pragma once
#include "munit/munit.h"

MunitResult test_create_host_func_null();
MunitResult test_create_host_func();
MunitResult test_call_host_func();
MunitResult test_write_host_function_details();
MunitResult test_write_host_function_call();

static MunitTest host_func_tests[] = {
    {
        "/test_create_host_func_null", /* name */
        test_create_host_func_null,    /* test */
        NULL,                          /* setup */
        NULL,                          /* tear_down */
        MUNIT_TEST_OPTION_NONE,        /* options */
        NULL                           /* parameters */
    },
    {
        "/test_create_host_func", /* name */
        test_create_host_func,    /* test */
        NULL,                     /* setup */
        NULL,                     /* tear_down */
        MUNIT_TEST_OPTION_NONE,   /* options */
        NULL                      /* parameters */
    },
    {
        "/test_call_host_func",
        test_call_host_func,
        NULL,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL},
    {
        "/test_write_host_function_details", /* name */
        test_write_host_function_details,    /* test */
        NULL,                     /* setup */
        NULL,                     /* tear_down */
        MUNIT_TEST_OPTION_NONE,   /* options */
        NULL                      /* parameters */
    },
    {
        "/test_write_host_function_call", /* name */
        test_write_host_function_call,    /* test */
        NULL,                     /* setup */
        NULL,                     /* tear_down */
        MUNIT_TEST_OPTION_NONE,   /* options */
        NULL                      /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
