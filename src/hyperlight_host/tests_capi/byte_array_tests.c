#include "byte_array_tests.h"
#include "munit/munit.h"
#include "hyperlight_host.h"
#include "mem.h"
#include "stdint.h"
#include "stdio.h"

MunitResult test_byte_array_null_ptr()
{
    struct Context *ctx = context_new();
    Handle barr_ref = byte_array_new(ctx, NULL, 123);
    munit_assert_true(handle_get_status(barr_ref) == ValidError);
    const char *err_msg = handle_get_error_message(ctx, barr_ref);
    munit_assert_not_null(err_msg);
    free((char *)err_msg);

    handle_free(ctx, barr_ref);
    context_free(ctx);

    return MUNIT_OK;
}

MunitResult test_byte_array_lifecycle()
{
    struct Context *ctx = context_new();
    const uint8_t size = 100;
    uint8_t *mem = create_u8_mem(size, true);
    Handle barr_ref = byte_array_new(ctx, mem, size);
    free(mem);
    munit_assert_true(handle_get_status(barr_ref) == ValidOther);
    munit_assert_int(size, ==, byte_array_len(ctx, barr_ref));

    uint8_t *barr_ptr = byte_array_get(ctx, barr_ref);

    byte_array_raw_free(barr_ptr, size);
    handle_free(ctx, barr_ref);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_byte_array_new_from_file()
{
    const char *file_name = "./tests_capi/byte_array_tests.c";
    struct Context *ctx = context_new();

    Handle barr_ref = byte_array_new_from_file(ctx, file_name);
    munit_assert_true(handle_get_status(barr_ref) == ValidOther);
    munit_assert_true(byte_array_len(ctx, barr_ref) > 0);
    long actual_size = file_size(file_name);
    munit_assert_long(actual_size, ==, byte_array_len(ctx, barr_ref));

    handle_free(ctx, barr_ref);
    context_free(ctx);
    return MUNIT_OK;
}

long file_size(const char *fname)
{
    FILE *fp = fopen(fname, "rb");
    fseek(fp, 0, SEEK_END);
    long sz = ftell(fp);
    rewind(fp);
    fclose(fp);
    return sz;
}
