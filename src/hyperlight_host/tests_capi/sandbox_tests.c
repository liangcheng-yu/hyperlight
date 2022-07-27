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
    Handle sbox = sandbox_new(ctx, "nothing");
    handle_assert_no_error(ctx, sbox);

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
    Handle sbox = sandbox_new(ctx, bin_path);
    handle_assert_no_error(ctx, sbox);

    Handle bin_path_ret = guest_binary_path(ctx, sbox);
    handle_assert_no_error(ctx, bin_path_ret);
    const char *bin_path_str = handle_get_string(ctx, bin_path_ret);
    munit_assert_not_null(bin_path_str);
    munit_assert_string_equal(bin_path_str, bin_path);
    free((char *)bin_path_str);
    handle_free(ctx, bin_path_ret);
    handle_free(ctx, sbox);
    context_free(ctx);
    return MUNIT_OK;
}
