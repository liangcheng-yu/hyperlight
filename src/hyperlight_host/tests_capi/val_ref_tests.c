#include "munit/munit.h"
#include "hyperlight_host.h"
#include "stdlib.h"
#include "mem.h"
#include "val_ref_tests.h"
#include "val_ref.h"

MunitResult test_val_ref_new()
{
    const uint8_t num_creates = 100;
    for (size_t i = 0; i < num_creates; i++)
    {
        uint8_t len = i * 10;
        int8_t *mem = create_i8_mem(len, false);
        struct Val *val = val_ref_new(mem, len, Raw);
        free(mem);
        val_ref_free(val);
    }
    return MUNIT_OK;
}

MunitResult test_val_refs_compare()
{
    const uint8_t len = 80;
    {
        // two empty `ValRef`s should be equal
        const struct Val *val1 = val_ref_empty();
        const struct Val *val2 = val_ref_empty();
        munit_assert_true(val_refs_compare(val1, val2));
        val_ref_free((struct Val *)val2);
        val_ref_free((struct Val *)val1);
    }
    {
        // one empty and one non-empty `ValRef` should not be
        // equal
        int8_t *mem = create_i8_mem(len, false);
        const struct Val *val1 = val_ref_new(mem, len, Raw);
        free(mem);
        struct Val *val2 = val_ref_empty();
        munit_assert_false(val_refs_compare(val1, val2));
        val_ref_free((struct Val *)val1);
        val_ref_free((struct Val *)val2);
    }
    {
        // two non-empty `ValRef`s with the same data
        // and serialization type should be equal
        int8_t *mem1 = create_i8_mem(len, true);
        int8_t *mem2 = create_i8_mem(len, true);
        const struct Val *val1 = val_ref_new(mem1, len, Raw);
        const struct Val *val2 = val_ref_new(mem2, len, Raw);
        free(mem1);
        free(mem2);
        munit_assert_true(val_refs_compare(val1, val2));
        val_ref_free((struct Val *)val1);
        val_ref_free((struct Val *)val2);
    }
    {
        // two non-empty `ValRef`s with the same data and
        // different serialization types should not be
        // equal
        int8_t *mem1 = create_i8_mem(len, true);
        int8_t *mem2 = create_i8_mem(len, true);
        const struct Val *val1 = val_ref_new(mem1, len, Raw);
        const struct Val *val2 = val_ref_new(mem2, len, Json);
        free(mem1);
        free(mem2);
        munit_assert_false(val_refs_compare(val1, val2));
        val_ref_free((struct Val *)val1);
        val_ref_free((struct Val *)val2);
    }
    {
        // two non-empty `ValRef`s with the same copy of the same
        // data and the same serialization type should be
        // equal
        int8_t *mem = create_i8_mem(len, true);
        const struct Val *val1 = val_ref_new(mem, len, Raw);
        const struct Val *val2 = val_ref_new(mem, len, Raw);
        free(mem);
        munit_assert_true(val_refs_compare(val1, val2));
        val_ref_free((struct Val *)val1);
        val_ref_free((struct Val *)val2);
    }

    return MUNIT_OK;
}
