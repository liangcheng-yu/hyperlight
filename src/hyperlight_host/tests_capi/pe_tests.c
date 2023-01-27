#include "pe_tests.h"
#include "munit/munit.h"
#include "hyperlight_host.h"
#include "mem.h"
#include "stdio.h"
#include "stdint.h"
#include "pe_tests.h"
#include "err.h"

MunitResult test_pe_get_headers(void)
{
    for (size_t i = 0; i < NUM_PE_FILES; i++)
    {
        const char *filename = pe_filenames[i];
        munit_logf(MUNIT_LOG_INFO, "reading PE file %s\n", filename);

        Context *ctx = context_new();
        Handle barr_ref = byte_array_new_from_file(ctx, filename);
        handle_assert_no_error(ctx, barr_ref);

        Handle pe_ref = pe_parse(ctx, barr_ref);
        handle_assert_no_error(ctx, pe_ref);

        PEHeaders hdrs = pe_get_headers(ctx, pe_ref);
        munit_assert(hdrs.stack_reserve > 0);
        munit_assert(hdrs.stack_commit > 0);
        munit_assert(hdrs.heap_reserve > 0);
        munit_assert(hdrs.heap_commit > 0);
        munit_assert(hdrs.entrypoint_offset > 0);
        munit_assert(hdrs.preferred_load_address > 0);

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
            munit_logf(MUNIT_LOG_INFO, "PE file: %s\n", pe_filename);

            Context *ctx = context_new();
            Handle mem_ref = byte_array_new_from_file(ctx, pe_filename);
            handle_assert_no_error(ctx, mem_ref);

            // Remember the original payload
            uint8_t *orig_bytes = byte_array_get(ctx, mem_ref);
            int64_t orig_len = byte_array_len(ctx, mem_ref);

            Handle pe_ref = pe_parse(ctx, mem_ref);
            handle_assert_no_error(ctx, pe_ref);

            Handle result_ref = pe_relocate(
                ctx,
                pe_ref,
                mem_ref,
                123);
            handle_assert_no_error(ctx, result_ref);
            HandleStatus result_status = handle_get_status(result_ref);

            if (result_status == ValidOther)
            {
                // check that memory has been modified from `pe_file.data`
                int64_t reloc_len = byte_array_len(ctx, mem_ref);
                if (orig_len != reloc_len)
                {
                    munit_error("the relocated pe file should be the same size as the original");
                }

                uint8_t *reloc_bytes = byte_array_get(ctx, mem_ref);
                munit_assert_memory_not_equal(orig_len, orig_bytes, reloc_bytes);
                byte_array_free(reloc_bytes, reloc_len);
                handle_free(ctx, result_ref);
            }
            else if (result_status != ValidEmpty)
            {
                munit_errorf("expected a relocate that does nothing to return ValidEmpty but got %d", result_status);
            }

            byte_array_free(orig_bytes, orig_len);
            handle_free(ctx, mem_ref);
            context_free(ctx);
        }
    }
    return MUNIT_OK;
}
