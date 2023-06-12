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

MunitResult test_hyperv_linux_execute_until_halt(const MunitParameter params[], void *fixture)
{
    CHECK_HYPERV_LINUX_PRESENT;
    const size_t ACTUAL_MEM_SIZE = 0x4000;
    const size_t REGION_MEM_SIZE = 0x1000;
    const uint8_t CODE[] = {
        0xba, 0xf8, 0x03, /* mov $0x3f8, %dx */
        0x00, 0xd8,       /* add %bl, %al */
        0x04, '0',        /* add $'0', %al */
        0xee,             /* out %al, (%dx) */
        0xb0, '\0',       /* mov $'\n', %al */
        0xee,             /* out %al, (%dx) */
        0xf4,             /* hlt */
    };
    const uint8_t CODE_LENGTH = sizeof(CODE);
    struct Context *ctx = context_new();
    Handle shared_mem_ref = shared_memory_new(ctx, ACTUAL_MEM_SIZE);

    {
        // copy code into guest memory
        Handle barr_ref = byte_array_new(ctx, CODE, CODE_LENGTH);
        handle_assert_no_error(ctx, barr_ref);
        Handle copy_res_ref = shared_memory_copy_from_byte_array(ctx, shared_mem_ref, barr_ref, 0, 0, CODE_LENGTH);
        handle_assert_no_error(ctx, copy_res_ref);
        handle_free(ctx, barr_ref);
    }

    HypervisorAddrs addrs = {
        .entrypoint = 0x1000,
        .guest_pfn = 0x1,
        .host_addr = shared_memory_get_address(ctx, shared_mem_ref),
        .mem_size = REGION_MEM_SIZE};
    Handle driver_ref = hyperv_linux_create_driver_simple(ctx, false, addrs);
    handle_assert_no_error(ctx, driver_ref);
    Handle apply_regs_ref = hyperv_linux_apply_registers(ctx, driver_ref);
    handle_assert_no_error(ctx, apply_regs_ref);
    {
        // TODO: create and register outb and mem access func handles
        Handle outb_func_ref = outb_fn_handler_create(ctx, outb_func);
        handle_assert_no_error(ctx, outb_func_ref);
        Handle mem_access_func_ref = mem_access_handler_create(ctx, mem_access_func);
        handle_assert_no_error(ctx, mem_access_func_ref);
        Handle exec_res_ref = hyperv_linux_execute_until_halt(
            ctx,
            driver_ref,
            outb_func_ref,
            mem_access_func_ref);
        handle_assert_no_error(ctx, exec_res_ref);
        // a valid execution should return an empty handle
        munit_assert_true(handle_get_status(exec_res_ref) == ValidEmpty);
        handle_free(ctx, exec_res_ref);
        handle_free(ctx, outb_func_ref);
        handle_free(ctx, mem_access_func_ref);
    }
    handle_free(ctx, driver_ref);
    handle_free(ctx, shared_mem_ref);
    context_free(ctx);
    return MUNIT_OK;
}

#endif
