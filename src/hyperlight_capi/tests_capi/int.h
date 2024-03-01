#pragma once
#include "munit/munit.h"

MunitResult test_int_64(const MunitParameter params[], void *user_data);
MunitResult test_int_32(const MunitParameter params[], void *user_data);

static MunitTest int_handle_tests[] = {
    {
        (char *)"/test_int_64", /* name */
        test_int_64,            /* test */
        NULL,                   /* setup */
        NULL,                   /* tear_down */
        MUNIT_TEST_OPTION_NONE, /* options */
        NULL                    /* parameters */
    },
    {
        (char *)"/test_int_32", /* name */
        test_int_32,            /* test */
        NULL,                   /* setup */
        NULL,                   /* tear_down */
        MUNIT_TEST_OPTION_NONE, /* options */
        NULL                    /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
