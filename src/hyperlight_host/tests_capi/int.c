#include "int.h"
#include "munit/munit.h"
#include "hyperlight_host.h"

MunitResult test_int_64()
{
    const int64_t val = 6400;
    Context *ctx = context_new();
    Handle ref = int_64_new(ctx, val);
    munit_assert_true(handle_is_int_64(ctx, ref));
    munit_assert_int(val, ==, handle_get_int_64(ctx, ref));
    handle_free(ctx, ref);
    context_free(ctx);
    return MUNIT_OK;
}
MunitResult test_int_32()
{
    const int32_t val = 3200;
    Context *ctx = context_new();
    Handle ref = int_32_new(ctx, val);
    munit_assert_true(handle_is_int_32(ctx, ref));
    munit_assert_int(val, ==, handle_get_int_32(ctx, ref));
    context_free(ctx);
    return MUNIT_OK;
}
