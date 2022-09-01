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
MunitResult test_open_mshv(const MunitParameter params[], void *fixture);
MunitResult test_create_vm(const MunitParameter params[], void *fixture);
MunitResult test_create_vcpu(const MunitParameter params[], void *fixture);
MunitResult test_map_user_memory_region(const MunitParameter params[], void *fixture);
MunitResult test_set_registers(const MunitParameter params[], void *fixture);
MunitResult test_run_vpcu(const MunitParameter params[], void *fixture);
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
    {"/test_open_mshv",
     test_open_mshv,
     hyperv_linux_set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_create_vm",
     test_create_vm,
     hyperv_linux_set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_create_vcpu",
     test_create_vcpu,
     hyperv_linux_set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_map_user_memory_region",
     test_map_user_memory_region,
     hyperv_linux_set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_set_registers",
     test_set_registers,
     hyperv_linux_set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_run_vcpu",
     test_run_vpcu,
     hyperv_linux_set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
#endif
