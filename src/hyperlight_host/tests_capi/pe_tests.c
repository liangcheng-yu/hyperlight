#include "pe_tests.h"
#include "munit/munit.h"
#include "hyperlight_host.h"
#include "mem.h"
#include "stdio.h"
#include "stdint.h"
#include "pe_tests.h"
#include "err.h"

MunitResult test_pe_getters(void)
{
    for (size_t i = 0; i < NUM_PE_FILES; i++)
    {
        const char *filename = pe_filenames[i];
        printf("reading PE file %s\n", filename);

        Context *ctx = context_new();
        Handle barr_ref = byte_array_new_from_file(ctx, filename);
        handle_assert_no_error(ctx, barr_ref);

        Handle pe_ref = pe_parse(ctx, barr_ref);
        handle_assert_no_error(ctx, pe_ref);

        munit_assert(pe_stack_reserve(ctx, pe_ref) > 0);
        munit_assert(pe_stack_commit(ctx, pe_ref) > 0);
        munit_assert(pe_heap_reserve(ctx, pe_ref) > 0);
        munit_assert(pe_heap_commit(ctx, pe_ref) > 0);
        munit_assert(pe_entry_point_offset(ctx, pe_ref) > 0);

        handle_free(ctx, pe_ref);
        handle_free(ctx, barr_ref);
        context_free(ctx);
    }

    return MUNIT_OK;
}

MunitResult test_pe_relocate()
{
    {
        // invalid PE file
        Context *ctx = context_new();
        const uint8_t mem_len = 100;
        uint8_t *mem = create_u8_mem(mem_len, true);
        Handle mem_ref = byte_array_new(ctx, mem, mem_len);
        free(mem);
        Handle pe_ref = pe_parse(ctx, mem_ref);
        handle_assert_error(ctx, pe_ref);
        Handle ret_ref = pe_relocate(ctx, pe_ref, mem_ref, 0);
        handle_assert_error(ctx, ret_ref);
        handle_free(ctx, mem_ref);
        handle_free(ctx, pe_ref);
        handle_free(ctx, ret_ref);
        context_free(ctx);
    }
    {
        // real PE files
        for (size_t file_num = 0; file_num < NUM_PE_FILES; file_num++)
        {
            const char *pe_filename = pe_filenames[file_num];
            printf("PE file: %s\n", pe_filename);

            Context *ctx = context_new();
            Handle mem_ref = byte_array_new_from_file(ctx, pe_filename);
            handle_assert_no_error(ctx, mem_ref);
            Handle pe_ref = pe_parse(ctx, mem_ref);
            handle_assert_no_error(ctx, pe_ref);

            Handle ret_ref = pe_relocate(
                ctx,
                pe_ref,
                mem_ref,
                123);
            handle_assert_no_error(ctx, ret_ref);
            munit_assert_true(handles_equal(mem_ref, ret_ref));
            // TODO: check that memory has been modified from `pe_file.data`

            handle_free(ctx, mem_ref);
            context_free(ctx);
        }
    }
    return MUNIT_OK;
}
