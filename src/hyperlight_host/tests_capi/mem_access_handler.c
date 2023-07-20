#include "outb_handler.h"
#include "hyperlight_host.h"
#include "err.h"

void mem_access_handler_func()
{
    return;
}

MunitResult test_mem_access_handler_create(const MunitParameter params[], void *fixture)
{
    Context *ctx = context_new("test correlation id");
    Handle create_res = mem_access_handler_create(ctx, mem_access_handler_func);

    handle_assert_no_error(ctx, create_res);

    handle_free(ctx, create_res);
    context_free(ctx);
    return MUNIT_OK;
}
MunitResult test_mem_access_handler_call(const MunitParameter params[], void *fixture)
{
    Context *ctx = context_new("test correlation id");

    Handle fn_ref = mem_access_handler_create(ctx, mem_access_handler_func);
    handle_assert_no_error(ctx, fn_ref);

    {
        // the first call should succeed
        Handle call_res_ref_1 = mem_access_handler_call(ctx, fn_ref);
        handle_assert_no_error(ctx, call_res_ref_1);
    }

    handle_free(ctx, fn_ref);

    {
        // after we free the function Handle, calling the function
        // should fail
        Handle call_res_ref_2 = mem_access_handler_call(ctx, fn_ref);
        handle_assert_error(ctx, call_res_ref_2);
    }

    context_free(ctx);
    return MUNIT_OK;
}
