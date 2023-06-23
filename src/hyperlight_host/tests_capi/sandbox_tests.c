#include "hyperlight_host.h"
#include "munit/munit.h"
#include "sandbox_tests.h"
#include "err.h"
#include "mem_mgr_tests.h"

MunitResult test_is_hypervisor_present()
{
    bool is_present = is_hypervisor_present();
    munit_assert_true(is_present);
    return MUNIT_OK;
}

void host_print(const char *str)
{
    munit_assert_string_equal(str, "Hello, world!");
}

MunitResult test_host_print()
{
    Context *ctx = context_new();
    SandboxMemoryConfiguration mem_cfg = {
        .guest_error_buffer_size = 4096,
        .host_function_definition_size = 4096,
        .input_data_size = 4096,
        .output_data_size = 4096,
        .host_exception_size = 4096};
#ifdef DEBUG
    Handle binary = string_new(ctx, "../tests/Hyperlight.Tests/bin/debug/net6.0/simpleguest.exe");
#else
    Handle binary = string_new(ctx, "../tests/Hyperlight.Tests/bin/release/net6.0/simpleguest.exe");
#endif
    handle_assert_no_error(ctx, binary);

    Handle sbx = sandbox_new(ctx, binary, &mem_cfg,0, host_print);
    handle_assert_no_error(ctx, sbx);

    sandbox_call_host_print(ctx, sbx, "Hello, world!");

    handle_free(ctx, binary);
    handle_free(ctx, sbx);
    context_free(ctx);
    return MUNIT_OK;
}