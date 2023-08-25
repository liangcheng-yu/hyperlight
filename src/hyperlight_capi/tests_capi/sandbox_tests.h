#pragma once

#include "munit/munit.h"
#include "hypervisor.h"

MunitResult test_is_hypervisor_present(const MunitParameter[], void *);
MunitResult test_host_print(const MunitParameter[], void *);

static MunitTest sandbox_tests[] = {
    {
        "/test_is_hypervisor_present", /* name */
        test_is_hypervisor_present,    /* test */
        hypervisor_check_flags,        /* setup */
        hypervisor_check_flags_teardown,                          /* tear_down */
        MUNIT_TEST_OPTION_NONE,        /* options */
        NULL                           /* parameters */
    },
    {
        "/test_host_print", /* name */
        test_host_print,    /* test */
        NULL,                          /* setup */
        NULL,                          /* tear_down */
        MUNIT_TEST_OPTION_NONE,        /* options */
        NULL                           /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
