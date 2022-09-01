#pragma once
#include "munit/munit.h"

MunitResult test_create_host_func();
MunitResult test_call_host_func();

static MunitTest host_func_tests[] = {
    {
        "/test_create_host_func", /* name */
        test_create_host_func,    /* test */
        NULL,                     /* setup */
        NULL,                     /* tear_down */
        MUNIT_TEST_OPTION_NONE,   /* options */
        NULL                      /* parameters */
    },
    {"/test_call_host_func",
     test_call_host_func,
     NULL,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
