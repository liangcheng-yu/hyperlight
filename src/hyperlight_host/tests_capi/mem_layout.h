#pragma once
#include "munit/munit.h"

MunitResult test_mem_layout_getters();

static MunitTest mem_layout_tests[] = {
    {
        "/test_mem_layout_getters", /* name */
        test_mem_layout_getters,    /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
