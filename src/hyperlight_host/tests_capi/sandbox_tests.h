#pragma once

#include "munit/munit.h"

MunitResult test_is_hypervisor_present();

static MunitTest sandbox_tests[] = {
    {
        "/test_is_hypervisor_present", /* name */
        test_is_hypervisor_present,    /* test */
        NULL,                          /* setup */
        NULL,                          /* tear_down */
        MUNIT_TEST_OPTION_NONE,        /* options */
        NULL                           /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
