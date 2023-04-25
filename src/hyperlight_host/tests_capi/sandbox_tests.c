#include "hyperlight_host.h"
#include "munit/munit.h"
#include "val_ref.h"
#include "callback.h"
#include "sandbox_tests.h"
#include "err.h"

MunitResult test_is_hypervisor_present()
{
    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle bin_path_ref = string_new(ctx, "nothing");
    Handle sbox = sandbox_new(ctx, bin_path_ref);
    handle_assert_no_error(ctx, sbox);
    handle_free(ctx, bin_path_ref);

    bool is_present = is_hypervisor_present(ctx, sbox);
    munit_assert_true(is_present);
    handle_free(ctx, sbox);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_get_binary_path()
{
    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    const char *bin_path = "./test_bin";
    Handle bin_path_ref_arg = string_new(ctx, bin_path);
    Handle sbox = sandbox_new(ctx, bin_path_ref_arg);
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
