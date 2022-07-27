#include "munit/munit.h"
#include "hyperlight_host.h"
#include "val_ref.h"
#include "context_tests.h"
#include "err.h"

// intended to create a lot of Handles without freeing them,
// making sure that context_free frees up all the memory created.
MunitResult test_context_contains_memory()
{
    Context *ctx = context_new();

    for (size_t i = 0; i < 10; i++)
    {
        Val *param_val = dummy_val_ref(10);
        munit_assert_not_null(param_val);
        Handle param_ref = val_ref_register(ctx, param_val);
        handle_assert_no_error(ctx, param_ref);
        val_ref_free(param_val);
        // NOTE: do not free param_ref here. it should be
        // cleaned up by the context_free call
        // below.
    }

    context_free(ctx);
    return MUNIT_OK;
}
