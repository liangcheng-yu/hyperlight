#include "string_tests.h"

MunitResult test_string_create_free()
{
    Context *ctx = context_new();
    const char *orig_str = "test_string_create_free test";
    Handle str_ref = string_new(ctx, orig_str);

    {
        const char *ret_str = handle_get_raw_string(ctx, str_ref);
        munit_assert_string_equal(orig_str, ret_str);
        free_raw_string(ret_str);
    }

    handle_free(ctx, str_ref);

    context_free(ctx);
    return MUNIT_OK;
}
