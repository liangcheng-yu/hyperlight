#include "munit/munit.h"
#include "hyperlight_host.h"
#include "context_tests.h"
#include "err.h"

// intended to create a lot of Handles without freeing them,
// making sure that context_free frees up all the memory created.
MunitResult test_context_contains_memory()
{
    Context *ctx = context_new("test correlation id");

    for (size_t i = 0; i < 10; i++)
    {
        Handle err_ref = handle_new_err(ctx, "this is an error!");
        handle_assert_error(ctx, err_ref);
        // NOTE: do not free err_ref here. it should be
        // cleaned up by the context_free call
        // below.
    }

    context_free(ctx);
    return MUNIT_OK;
}
