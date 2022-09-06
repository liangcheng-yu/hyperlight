#pragma once
#include "munit/munit.h"
#include <stdbool.h>
#if defined(__linux__)

static bool EXPECT_KVM_PRESENT = false;

#define CHECK_KVM_PRESENT  \
    if (!kvm_is_present()) \
    {                      \
        return MUNIT_SKIP; \
    };

MunitResult test_is_kvm_present(const MunitParameter params[], void *fixture);
MunitResult test_kvm_open(const MunitParameter params[], void *fixture);
MunitResult test_kvm_create_vm(const MunitParameter params[], void *fixture);
MunitResult test_kvm_create_vcpu(const MunitParameter params[], void *fixture);
MunitResult test_kvm_map_user_memory_region(const MunitParameter params[], void *fixtureid);
MunitResult test_kvm_set_registers(const MunitParameter params[], void *fixture);
MunitResult test_kvm_run_vpcu(const MunitParameter params[], void *fixture);
void *kvm_set_flags(const MunitParameter params[], void *user_data);

static MunitTest kvm_tests[] = {
    {
        "/test_is_kvm_present", // name
        test_is_kvm_present,    // test
        kvm_set_flags,          // setup
        NULL,                   // tear_down
        MUNIT_TEST_OPTION_NONE, // options
        NULL                    // parameters
    },
    {
        "/test_kvm_open",       // name
        test_kvm_open,          // test
        kvm_set_flags,          // setup
        NULL,                   // tear_down
        MUNIT_TEST_OPTION_NONE, // options
        NULL                    // parameters
    },
    {
        "/test_kvm_create_vm",  // name
        test_kvm_create_vm,     // test
        kvm_set_flags,          // setup
        NULL,                   // tear_down
        MUNIT_TEST_OPTION_NONE, // options
        NULL                    // parameters
    },
    {
        "/test_kvm_create_vcpu", // name
        test_kvm_create_vcpu,    // test
        kvm_set_flags,           // setup
        NULL,                    // tear_down
        MUNIT_TEST_OPTION_NONE,  // options
        NULL                     // parameters
    },
    {
        "/test_kvm_map_user_memory_region", // name
        test_kvm_map_user_memory_region,    // test
        kvm_set_flags,                      // setup
        NULL,                               // tear_down
        MUNIT_TEST_OPTION_NONE,             // options
        NULL                                // parameters
    },
    {
        "/test_kvm_set_registers", // name
        test_kvm_set_registers,    // test
        kvm_set_flags,             // setup
        NULL,                      // tear_down
        MUNIT_TEST_OPTION_NONE,    // options
        NULL                       // parameters
    },
    {
        "/test_kvm_run_vpcu",   // name
        test_kvm_run_vpcu,      // test
        kvm_set_flags,          // setup
        NULL,                   // tear_down
        MUNIT_TEST_OPTION_NONE, // options
        NULL                    // parameters
    },
};

#endif
