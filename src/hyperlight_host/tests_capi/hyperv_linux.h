#pragma once
#include "munit/munit.h"
#include <stdbool.h>

static bool EXPECT_PRERELEASE_API = true;
static bool EXPECT_HYPERVISOR_PRESENT = false;

#define CHECK_IF_HYPERVISOR_PRESENT                       \
    if (!is_hyperv_linux_present(!EXPECT_PRERELEASE_API)) \
    {                                                     \
        return MUNIT_OK;                                  \
    };

MunitResult test_is_hyperv_linux_present(const MunitParameter *, void *);
MunitResult test_open_mshv(const MunitParameter *, void *);
MunitResult test_create_vm(const MunitParameter *, void *);
MunitResult test_create_vcpu(const MunitParameter *, void *);
MunitResult test_map_user_memory_region(const MunitParameter *, void *);
MunitResult test_set_registers(const MunitParameter *, void *);
MunitResult test_run_vpcu(const MunitParameter *, void *);
void *set_flags(const MunitParameter params[], void *user_data);
bool get_flag_value(char *flag_value);

#if defined(__linux__)
static MunitTest hyperv_linux_tests[] = {
    {
        "/test_is_hyperv_linux_present", /* name */
        test_is_hyperv_linux_present,    /* test */
        set_flags,                       /* setup */
        NULL,                            /* tear_down */
        MUNIT_TEST_OPTION_NONE,          /* options */
        NULL                             /* parameters */
    },
    {"/test_open_mshv",
     test_open_mshv,
     set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_create_vm",
     test_create_vm,
     set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_create_vcpu",
     test_create_vcpu,
     set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_map_user_memory_region",
     test_map_user_memory_region,
     set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_set_registers",
     test_set_registers,
     set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {"/test_run_vcpu",
     test_run_vpcu,
     set_flags,
     NULL,
     MUNIT_TEST_OPTION_NONE,
     NULL},
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
#endif
