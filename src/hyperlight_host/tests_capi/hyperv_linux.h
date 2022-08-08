#pragma once
#include "munit/munit.h"
#include <stdbool.h>

#define CHECK_IF_HYPERVISOR_PRESENT \
if (!check_if_hypervisor_is_present())\
{\
    return MUNIT_OK;\
};

static bool EXPECT_PRERELEASE_API = true;
static bool EXPECT_HYPERVISOR_PRESENT = false;

MunitResult test_is_hyperv_linux_present(void);
MunitResult test_open_mshv(void);
MunitResult test_create_vm(void);
MunitResult test_create_vcpu(void);
MunitResult test_map_user_memory_region(void);
MunitResult test_set_registers(void);
MunitResult test_run_vpcu(void);
void set_flags(void);
bool get_flag_value(char* flag_value);
