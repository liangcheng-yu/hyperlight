#pragma once

#include "munit/munit.h"

MunitResult test_is_hypervisor_present();
MunitResult test_get_binary_path();

static MunitTest sandbox_tests[] = {
    {
        "/test_is_hypervisor_present", /* name */
        test_is_hypervisor_present,    /* test */
        NULL,                          /* setup */
        NULL,                          /* tear_down */
        MUNIT_TEST_OPTION_NONE,        /* options */
        NULL                           /* parameters */
    },
    {"/test_get_binary_path",
     test_get_binary_path,
     NULL,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
