#include "hyperlight_host.h"
#include "munit/munit.h"
#include "val_ref.h"
#include "callback.h"
#include "host_func_tests.h"
#include "err.h"
#include "stdio.h"

MunitResult test_create_host_func_null()
{
    Context *ctx = context_new();
    Callback *cb = NULL;
    Handle host_func_hdl = host_func_create(ctx, cb);
    handle_assert_error(ctx, host_func_hdl);
    handle_free(ctx, host_func_hdl);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_create_host_func()
{
    Context *ctx = context_new();
    Callback cb = {.func = test_callback};
    Handle host_func_ref = host_func_create(ctx, &cb);
    handle_assert_no_error(ctx, host_func_ref);
    handle_free(ctx, host_func_ref);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_call_host_func()
{
    Context *ctx = context_new();
    Handle sbox = sandbox_new(ctx, "some_bin");
    const char *host_func_name_1 = "test_func1";
    const char *host_func_name_2 = "test_func2";
    const Callback cb = {.func = test_callback};
    Handle host_func_ref_1 = host_func_create(ctx, &cb);
    handle_assert_no_error(ctx, host_func_ref_1);
    Handle host_func_ref_2 = host_func_create(ctx, &cb);
    handle_assert_no_error(ctx, host_func_ref_2);
    Handle host_func_1_hdl = host_func_register(
        ctx,
        sbox,
        host_func_name_1,
        host_func_ref_1);
    handle_assert_no_error(ctx, host_func_1_hdl);

    Handle host_func_2_hdl = host_func_register(
        ctx,
        sbox,
        host_func_name_2,
        host_func_ref_2);
    handle_assert_no_error(ctx, host_func_2_hdl);

    // test call host func 1
    {
        Val *param = dummy_val_ref(10);
        munit_assert_not_null(param);
        Handle param_ref = val_ref_register(ctx, param);
        handle_assert_no_error(ctx, param_ref);
        val_ref_free(param);
        Handle return_ref = host_func_call(ctx, sbox, host_func_name_1, param_ref);
        handle_assert_no_error(ctx, return_ref);
        struct Val *return_val = val_ref_get(ctx, return_ref);
        munit_assert_not_null(return_val);

        Val *expected_ret = dummy_val_ref(10);

        munit_assert_true(val_refs_compare(return_val, expected_ret));
        val_ref_free(expected_ret);
        val_ref_free(return_val);
        handle_free(ctx, param_ref);
        handle_free(ctx, return_ref);
    }
    // test call host func 2
    {
        Val *param = dummy_val_ref(10);
        munit_assert_not_null(param);
        Handle param_ref = val_ref_register(ctx, param);
        handle_assert_no_error(ctx, param_ref);
        val_ref_free(param);

        Handle return_ref = host_func_call(ctx, sbox, host_func_name_2, param_ref);
        Val *return_val = val_ref_get(ctx, return_ref);
        Val *expected_ret = dummy_val_ref(10);

        munit_assert_true(val_refs_compare(return_val, expected_ret));

        val_ref_free(expected_ret);
        val_ref_free(return_val);

        handle_free(ctx, param_ref);
        handle_free(ctx, return_ref);
    }
    handle_free(ctx, host_func_ref_1);
    handle_free(ctx, host_func_ref_2);
    handle_free(ctx, host_func_1_hdl);
    handle_free(ctx, host_func_2_hdl);
    context_free(ctx);
    return MUNIT_OK;
}
