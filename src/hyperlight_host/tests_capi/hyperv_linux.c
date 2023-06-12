#if defined(__linux__)

#include "hyperv_linux.h"
#include "hyperlight_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <strings.h>
#include <sys/mman.h>
#include "err.h"
#include "munit/munit.h"
#include "flag.h"

void *hyperv_linux_set_flags(const MunitParameter params[], void *user_data)
{
    // Set env var HYPERV_SHOULD_BE_PRESENT to require hyperv to be present for this test.
    char *env_var = NULL;
    env_var = getenv("HYPERV_SHOULD_BE_PRESENT");
    munit_logf(MUNIT_LOG_INFO, "env var HYPERV_SHOULD_BE_PRESENT %s\n", env_var);

    if (env_var != NULL)
    {
        EXPECT_HYPERV_LINUX_PRESENT = get_flag_value(env_var);
    }

    // Set env var SHOULD_HAVE_STABLE_API to require a stable api for this test.
    env_var = NULL;
    env_var = getenv("SHOULD_HAVE_STABLE_API");
    munit_logf(MUNIT_LOG_INFO, "env var SHOULD_HAVE_STABLE_API %s\n", env_var);

    if (env_var != NULL)
    {
        EXPECT_HYPERV_LINUX_PRERELEASE_API = !get_flag_value(env_var);
    }

    munit_logf(MUNIT_LOG_INFO, "EXPECT_HYPERV_LINUX_PRESENT: %s\n", EXPECT_HYPERV_LINUX_PRESENT ? "true" : "false");
    munit_logf(MUNIT_LOG_INFO, "EXPECT_HYPERV_LINUX_PRERELEASE_API: %s\n", EXPECT_HYPERV_LINUX_PRERELEASE_API ? "true" : "false");
    return NULL;
}

MunitResult test_is_hyperv_linux_present(const MunitParameter params[], void *fixture)
{
    bool status = is_hyperv_linux_present(false);
    if (EXPECT_HYPERV_LINUX_PRESENT && EXPECT_HYPERV_LINUX_PRERELEASE_API)
    {
        munit_assert_true(status);
    }
    else
    {
        munit_assert_false(status);
    }

    status = is_hyperv_linux_present(true);
    if (EXPECT_HYPERV_LINUX_PRESENT && !EXPECT_HYPERV_LINUX_PRERELEASE_API)
    {
        munit_assert_true(status);
    }
    else
    {
        munit_assert_false(status);
    }

    return MUNIT_OK;
}

MunitResult test_hyperv_linux_create_driver(const MunitParameter params[], void *fixture)
{
    CHECK_HYPERV_LINUX_PRESENT;
    const size_t MEM_SIZE = 0x1000;
    Context *ctx = context_new();
    Handle shared_mem_ref = shared_memory_new(ctx, MEM_SIZE);
    struct HypervisorAddrs addrs = {
        .entrypoint = 0,
        .guest_pfn = 0,
        .host_addr = shared_memory_get_address(ctx, shared_mem_ref),
        .mem_size = MEM_SIZE,
    };

    Handle hv_driver_hdl = hyperv_linux_create_driver(ctx, false, addrs, 0, 0);
    handle_assert_no_error(ctx, hv_driver_hdl);

    handle_free(ctx, hv_driver_hdl);
    handle_free(ctx, shared_mem_ref);
    context_free(ctx);
    return MUNIT_OK;
}

void outb_func(uint16_t port, uint64_t payload)
{
}
void mem_access_func(void)
{
}

#endif
