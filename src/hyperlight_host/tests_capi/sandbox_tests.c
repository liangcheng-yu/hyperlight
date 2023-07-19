#include "hyperlight_host.h"
#include "munit/munit.h"
#include "sandbox_tests.h"
#include "err.h"

MunitResult test_is_hypervisor_present(const MunitParameter params[], void *fixture)
{

    // TODO: remove this once we have WHP hooked up the the Rust Sandbox

#ifdef _WIN32
    return MUNIT_SKIP;
#endif

    HypervisorAvailabilityType *hypervisorAvailability = (HypervisorAvailabilityType *)fixture;
    bool status = is_hypervisor_present();

    if ((hypervisorAvailability->expect_hyperv_linux_present && hypervisorAvailability->expect_hyperv_linux_prerelease_api) || hypervisorAvailability->expect_kvm_present || hypervisorAvailability->expect_whp_present)
    {
        munit_assert_true(status);
    }
    else
    {
        munit_assert_false(status);
    }

    // TODO: Test for a non pre release API version of hyperv on linux when it is available.

    return MUNIT_OK;
}

void host_print(const char *str)
{
    munit_assert_string_equal(str, "Hello, world!");
}

MunitResult test_host_print(const MunitParameter params[], void *fixture)
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

    Handle sbx = sandbox_new(ctx, binary, mem_cfg, 0, host_print);
    handle_assert_no_error(ctx, sbx);

    sandbox_call_host_print(ctx, sbx, "Hello, world!");

    handle_free(ctx, binary);
    handle_free(ctx, sbx);
    context_free(ctx);
    return MUNIT_OK;
}
