#include "hyperlight_host.h"
#include "munit/munit.h"
#include "sandbox_tests.h"
#include "err.h"
#include "mem_mgr_tests.h"

MunitResult test_is_hypervisor_present()
{
    bool is_present = is_hypervisor_present();
    munit_assert_true(is_present);
    return MUNIT_OK;
}