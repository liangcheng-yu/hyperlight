#pragma once

#include "munit/munit.h"

MunitResult test_context_contains_memory(const MunitParameter params[], void *user_data);

static MunitTest context_tests[] = {
    {
        (char *)"/test_context_contains_memory", /* name */
        test_context_contains_memory,            /* test */
        NULL,                                    /* setup */
        NULL,                                    /* tear_down */
        MUNIT_TEST_OPTION_NONE,                  /* options */
        NULL                                     /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
