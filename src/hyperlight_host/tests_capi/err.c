#include "err.h"
#include "hyperlight_host.h"
#include "stdio.h"
#include "stdlib.h"
#include "munit/munit.h"

void handle_assert_error_impl(const Context *ctx, Handle hdl, const char *file, int line)
{
    if (!handle_is_error(hdl))
    {
        fprintf(stderr, "[%s:%d] expected error but got none\n", file, line);
        exit(1);
    }
}

void handle_assert_no_error_impl(const Context *ctx, Handle h, const char *file, int line)
{
    if (handle_is_error(h))
    {
        const char *err_msg = handle_get_error_message(ctx, h);
        fprintf(stderr, "[%s:%d] handle error: %s\n", file, line, err_msg);
        free((char *)err_msg);
        exit(1);
    }
}

MunitResult test_handle_is_empty()
{
    Context *ctx = context_new();
    Handle empty_ref = handle_new_empty();
    munit_assert_true(handle_is_empty(empty_ref));

    handle_free(ctx, empty_ref);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_handle_get_error_message()
{
    const char *err_msg = "test error message";
    Context *ctx = context_new();
    Handle err_ref = handle_new_err(ctx, err_msg);
    munit_assert_true(handle_is_error(err_ref));
    const char *actual_err_msg = handle_get_error_message(ctx, err_ref);
    munit_assert_string_equal(err_msg, actual_err_msg);
    free((char *)actual_err_msg);
    handle_free(ctx, err_ref);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_handle_new_error_null_ptr()
{
    Context *ctx = context_new();

    Handle errHdl = handle_new_err(ctx, NULL);
    munit_assert_true(handle_is_invalid(errHdl));

    handle_free(ctx, errHdl);
    context_free(ctx);

    return MUNIT_OK;
}
