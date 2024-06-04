#include "uint.h"
#include "munit/munit.h"
#include "hyperlight_capi.h"

MunitResult test_uint_64(const MunitParameter params[], void *user_data)
{
    const uint64_t val = 6400;
    Context *ctx = context_new("test correlation id");
    Handle ref = uint_64_new(ctx, val);
    munit_assert_true(handle_is_uint_64(ctx, ref));
    munit_assert_int(val, ==, handle_get_uint_64(ctx, ref));
    handle_free(ctx, ref);
    context_free(ctx);
    return MUNIT_OK;
}
MunitResult test_uint_32(const MunitParameter params[], void *user_data)
{
    const uint32_t val = 3200;
    Context *ctx = context_new("test correlation id");
    Handle ref = uint_32_new(ctx, val);
    munit_assert_true(handle_is_uint_32(ctx, ref));
    munit_assert_int(val, ==, handle_get_uint_32(ctx, ref));
    context_free(ctx);
    return MUNIT_OK;
}
