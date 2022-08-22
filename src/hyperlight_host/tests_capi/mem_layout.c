#include "mem_layout.h"
#include "hyperlight_host.h"
#include "munit/munit.h"

#define RUN_TEST_USIZE(actual, expected) \
    munit_assert_int(actual, ==, expected);

MunitResult test_mem_layout_getters(void)
{
    static const size_t code_size = 0x100;
    static const size_t stack_size = 0x1000;
    static const size_t heap_size = 0x5000;

    struct Context *ctx = context_new();
    struct Handle mem_cfg_ref = mem_config_new(ctx, 1, 2, 3, 4, 5);
    struct Handle mem_layout_ref = mem_layout_new(ctx, mem_cfg_ref, code_size, stack_size, heap_size);

    RUN_TEST_USIZE(mem_layout_get_stack_size(ctx, mem_layout_ref), stack_size);
    RUN_TEST_USIZE(mem_layout_get_heap_size(ctx, mem_layout_ref), heap_size);
    RUN_TEST_USIZE(mem_layout_get_code_size(ctx, mem_layout_ref), code_size);

    handle_free(ctx, mem_layout_ref);
    handle_free(ctx, mem_cfg_ref);
    context_free(ctx);
    return MUNIT_OK;
}
