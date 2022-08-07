#include "munit/munit.h"
#include "sandbox_tests.h"
#include "val_ref_tests.h"
#include "pe_tests.h"
#include "context_tests.h"
#include "host_func_tests.h"
#include "byte_array_tests.h"
#include "mem_config.h"
#include "mem_layout.h"
#include "hyperv_linux.h"

int main()
{
    // NOTE: do not use munit's suite functionality,
    // it leaks memory

    {
        // sandbox tests
        munit_assert_int(MUNIT_OK, ==, test_is_hypervisor_present());
        munit_assert_int(MUNIT_OK, ==, test_get_binary_path());
    }
    {
        // val ref tests
        munit_assert_int(MUNIT_OK, ==, test_val_ref_new());
        munit_assert_int(MUNIT_OK, ==, test_val_refs_compare());
    }
    {
        // host func tests
        munit_assert_int(MUNIT_OK, ==, test_create_host_func());
        munit_assert_int(MUNIT_OK, ==, test_call_host_func());
    }

    {
        // context tests
        munit_assert_int(MUNIT_OK, ==, test_context_contains_memory());
    }
    {
        // PE file tests
        munit_assert_int(MUNIT_OK, ==, test_pe_relocate());
        munit_assert_int(MUNIT_OK, ==, test_pe_getters());
    }
    {
        // byte array tests
        munit_assert_int(MUNIT_OK, ==, test_byte_array_lifecycle());
        munit_assert_int(MUNIT_OK, ==, test_byte_array_new_from_file());
    }
    {
        // mem config tests
        munit_assert_int(MUNIT_OK, ==, test_mem_config_getters());
    }
    {
        // mem layout tests
        munit_assert_int(MUNIT_OK, ==, test_mem_layout_get());
    }
    {
        // hyperv on linux tests
        set_flags();
        munit_assert_int(MUNIT_OK, ==,  test_is_hyperv_linux_present());
        munit_assert_int(MUNIT_OK, ==,  test_open_mshv());
        munit_assert_int(MUNIT_OK, ==,  test_create_vm());
        munit_assert_int(MUNIT_OK, ==,  test_create_vcpu());
        munit_assert_int(MUNIT_OK, ==,  test_map_user_memory_region());
        munit_assert_int(MUNIT_OK, ==,  test_set_registers());
    }
}
