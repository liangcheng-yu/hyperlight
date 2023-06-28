#pragma once
#include "munit/munit.h"
#include <stdbool.h>
#include "hypervisor.h"

#if defined(__linux__)

MunitResult test_hyperv_linux_create_driver(const MunitParameter[], void *);
MunitResult test_is_hyperv_linux_present(const MunitParameter[], void *);

static MunitTest hyperv_linux_tests[] = {
    {
        "/test_is_hyperv_linux_present", /* name */
        test_is_hyperv_linux_present,    /* test */
        hypervisor_check_flags,             /* setup */
        hypervisor_check_flags_teardown,                               /* tear_down */
        MUNIT_TEST_OPTION_NONE,             /* options */
        NULL                                /* parameters */
    },
    {
        "/test_hyperv_linux_create_driver", /* name */
        test_hyperv_linux_create_driver,    /* test */
        hypervisor_check_flags,             /* setup */
        hypervisor_check_flags_teardown,                               /* tear_down */
        MUNIT_TEST_OPTION_NONE,             /* options */
        NULL                                /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
#endif
