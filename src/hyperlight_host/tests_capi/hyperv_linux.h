#pragma once
#include "munit/munit.h"
#include <stdbool.h>

#if defined(__linux__)

static bool EXPECT_HYPERV_LINUX_PRERELEASE_API = true;
static bool EXPECT_HYPERV_LINUX_PRESENT = false;

#define CHECK_HYPERV_LINUX_PRESENT                                     \
    if (!is_hyperv_linux_present(!EXPECT_HYPERV_LINUX_PRERELEASE_API)) \
    {                                                                  \
        return MUNIT_SKIP;                                             \
    };

MunitResult test_is_hyperv_linux_present(const MunitParameter[], void *);
MunitResult test_hyperv_linux_create_driver(const MunitParameter[], void *);
MunitResult test_hyperv_linux_execute_until_halt(const MunitParameter params[], void *fixture);

void *hyperv_linux_set_flags(const MunitParameter params[], void *user_data);

static MunitTest hyperv_linux_tests[] = {
    {
        "/test_is_hyperv_linux_present", /* name */
        test_is_hyperv_linux_present,    /* test */
        hyperv_linux_set_flags,          /* setup */
        NULL,                            /* tear_down */
        MUNIT_TEST_OPTION_NONE,          /* options */
        NULL                             /* parameters */
    },
    {
        "/test_hyperv_linux_create_driver", /* name */
        test_hyperv_linux_create_driver,    /* test */
        hyperv_linux_set_flags,             /* setup */
        NULL,                               /* tear_down */
        MUNIT_TEST_OPTION_NONE,             /* options */
        NULL                                /* parameters */
    },
    {
        "/test_hyperv_linux_execute_until_halt", /* name */
        test_hyperv_linux_execute_until_halt,    /* test */
        hyperv_linux_set_flags,                  /* setup */
        NULL,                                    /* tear_down */
        MUNIT_TEST_OPTION_NONE,                  /* options */
        NULL                                     /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
#endif
