#include "mem_config.h"
#include "hyperlight_host.h"
#include "munit/munit.h"
#include "stdint.h"

MunitResult test_mem_config_getters(void)
{
    Context *ctx = context_new();
    static const uintptr_t input_data_size = 1;
    static const uintptr_t output_data_size = 10;
    static const uintptr_t function_definition_size = 100;
    static const uintptr_t host_exception_size = 101;
    static const uintptr_t guest_error_message_size = 102;

    Handle mem_config_ref = mem_config_new(ctx,
                                           input_data_size,
                                           output_data_size,
                                           function_definition_size,
                                           host_exception_size,
                                           guest_error_message_size);
    // SandboxMemoryConfiguration::MIN_INPUT_SIZE
    munit_assert_int(0x2000, ==, mem_config_get_input_data_size(ctx, mem_config_ref));
    // SandboxMemoryConfiguration::MIN_OUTPUT_SIZE
    munit_assert_int(0x2000, ==, mem_config_get_output_data_size(ctx, mem_config_ref));
    // SandboxMemoryConfiguration::MIN_GUEST_ERROR_MESSAGE_SIZE
    munit_assert_int(0x80, ==, mem_config_get_guest_error_message_size(ctx, mem_config_ref));
    // SandboxMemoryConfiguration::MIN_HOST_FUNCTION_DEFINITION_SIZE
    munit_assert_int(0x400, ==, mem_config_host_function_definition_size(ctx, mem_config_ref));
    // SandboxMemoryConfiguration::MIN_HOST_EXCEPTION_SIZE
    munit_assert_int(0x400, ==, mem_config_host_exception_size(ctx, mem_config_ref));
    handle_free(ctx, mem_config_ref);

    mem_config_ref = mem_config_new(ctx, 0x2001, 0x2001, 0x2001, 0x2001, 0x2001);
    munit_assert_int(0x2001, ==, mem_config_get_input_data_size(ctx, mem_config_ref));
    munit_assert_int(0x2001, ==, mem_config_get_output_data_size(ctx, mem_config_ref));
    munit_assert_int(0x2001, ==, mem_config_get_guest_error_message_size(ctx, mem_config_ref));
    munit_assert_int(0x2001, ==, mem_config_host_function_definition_size(ctx, mem_config_ref));
    munit_assert_int(0x2001, ==, mem_config_host_exception_size(ctx, mem_config_ref));
    handle_free(ctx, mem_config_ref);
    context_free(ctx);

    return MUNIT_OK;
}
