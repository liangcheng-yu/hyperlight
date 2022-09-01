#pragma once

#include "munit/munit.h"

MunitResult test_val_ref_new();
MunitResult test_val_refs_compare();

static MunitTest val_ref_tests[] = {
    {
        "/test_val_ref_new",    /* name */
        test_val_ref_new,       /* test */
        NULL,                   /* setup */
        NULL,                   /* tear_down */
        MUNIT_TEST_OPTION_NONE, /* options */
        NULL                    /* parameters */
    },
    {"/test_val_refs_compare",
     test_val_refs_compare,
     NULL,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
