#include "hyperlight_host.h"
#include "munit/munit.h"
#include "val_ref.h"
#include "callback.h"
#include "sandbox_tests.h"
#include "err.h"
#include "mem_mgr_tests.h"

MunitResult test_is_hypervisor_present()
{
    bool is_present = is_hypervisor_present();
    munit_assert_true(is_present);
    return MUNIT_OK;
}

MunitResult test_get_binary_path()
{
    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    const char *bin_path = "./test_bin";
    Handle bin_path_ref_arg = string_new(ctx, bin_path);
    Handle mem_mgr_ref = new_mem_mgr(ctx);
    handle_assert_no_error(ctx, mem_mgr_ref);
    Handle sbox = sandbox_new(ctx, bin_path_ref_arg, mem_mgr_ref);
    handle_free(ctx, mem_mgr_ref);
    handle_assert_no_error(ctx, sbox);
    handle_free(ctx, bin_path_ref_arg);

    {
        Handle bin_path_ref = guest_binary_path(ctx, sbox);
        handle_assert_no_error(ctx, bin_path_ref);
        const char *bin_path_str = handle_get_raw_string(ctx, bin_path_ref);
        munit_assert_not_null(bin_path_str);
        munit_assert_string_equal(bin_path_str, bin_path);
        free_raw_string(bin_path_str);
        handle_free(ctx, bin_path_ref);
    }

    handle_free(ctx, sbox);
    context_free(ctx);
    return MUNIT_OK;
}
