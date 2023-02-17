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
    struct SandboxMemoryConfiguration mem_cfg = {
        .guest_error_buffer_size = 1,
        .host_function_definition_size = 2,
        .input_data_size = 3,
        .output_data_size = 4,
        .host_exception_size = 5};
    struct Handle mem_layout_ref = mem_layout_new(ctx, mem_cfg, code_size, stack_size, heap_size);

    RUN_TEST_USIZE(mem_layout_get_stack_size(ctx, mem_layout_ref), stack_size);

    handle_free(ctx, mem_layout_ref);
    context_free(ctx);
    return MUNIT_OK;
}
