#include "hyperlight_host.h"
#include "munit/munit.h"
#include "val_ref.h"
#include "callback.h"
#include "host_func_tests.h"
#include "err.h"
#include "stdio.h"
#include "mem_mgr_tests.h"

MunitResult test_create_host_func_null()
{
    Context *ctx = context_new();
    {
        Handle host_func_hdl = host_func_create(ctx, NULL);
        handle_assert_error(ctx, host_func_hdl);
        const char *err_msg = handle_get_error_message(ctx, host_func_hdl);
        munit_assert_string_equal("NULL callback func", err_msg);
        free((void *)err_msg);
        handle_free(ctx, host_func_hdl);
    }

    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_create_host_func()
{
    Context *ctx = context_new();
    Handle host_func_ref = host_func_create(ctx, test_callback);
    handle_assert_no_error(ctx, host_func_ref);
    handle_free(ctx, host_func_ref);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_call_host_func()
{
    Context *ctx = context_new();
    Handle bin_path_ref = string_new(ctx, "some_bin");
    Handle mem_mgr_ref = new_mem_mgr(ctx);
    handle_assert_no_error(ctx, mem_mgr_ref);
    Handle sbox = sandbox_new(ctx, bin_path_ref, mem_mgr_ref);
    handle_assert_no_error(ctx, sbox);
    handle_free(ctx, mem_mgr_ref);
    handle_free(ctx, bin_path_ref);
    const char *host_func_name_1 = "test_func1";
    const char *host_func_name_2 = "test_func2";
    Handle host_func_ref_1 = host_func_create(ctx, test_callback);
    handle_assert_no_error(ctx, host_func_ref_1);
    Handle host_func_ref_2 = host_func_create(ctx, test_callback);
    handle_assert_no_error(ctx, host_func_ref_2);
    Handle host_func_1_hdl = host_func_register(
        ctx,
        sbox,
        host_func_name_1,
        host_func_ref_1);
    handle_assert_no_error(ctx, host_func_1_hdl);

    Handle host_func_2_hdl = host_func_register(
        ctx,
        sbox,
        host_func_name_2,
        host_func_ref_2);
    handle_assert_no_error(ctx, host_func_2_hdl);

    // test call host func 1
    {
        Val *param = dummy_val_ref(10);
        munit_assert_not_null(param);
        Handle param_ref = val_ref_register(ctx, param);
        handle_assert_no_error(ctx, param_ref);
        val_ref_free(param);
        Handle return_ref = host_func_call(ctx, sbox, host_func_name_1, param_ref);
        handle_assert_no_error(ctx, return_ref);
        struct Val *return_val = val_ref_get(ctx, return_ref);
        munit_assert_not_null(return_val);

        Val *expected_ret = dummy_val_ref(10);

        munit_assert_true(val_refs_compare(return_val, expected_ret));
        val_ref_free(expected_ret);
        val_ref_free(return_val);
        handle_free(ctx, param_ref);
        handle_free(ctx, return_ref);
    }
    // test call host func 2
    {
        Val *param = dummy_val_ref(10);
        munit_assert_not_null(param);
        Handle param_ref = val_ref_register(ctx, param);
        handle_assert_no_error(ctx, param_ref);
        val_ref_free(param);

        Handle return_ref = host_func_call(ctx, sbox, host_func_name_2, param_ref);
        Val *return_val = val_ref_get(ctx, return_ref);
        Val *expected_ret = dummy_val_ref(10);

        munit_assert_true(val_refs_compare(return_val, expected_ret));

        val_ref_free(expected_ret);
        val_ref_free(return_val);

        handle_free(ctx, param_ref);
        handle_free(ctx, return_ref);
    }
    handle_free(ctx, host_func_ref_1);
    handle_free(ctx, host_func_ref_2);
    handle_free(ctx, host_func_1_hdl);
    handle_free(ctx, host_func_2_hdl);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_write_host_function_details()
{

    Context *ctx = context_new();
    size_t code_size = 4096;
    size_t stack_size = 4096;
    size_t heap_size = 4096;

    SandboxMemoryConfiguration mem_cfg = {
        .guest_error_buffer_size = 4096,
        .host_function_definition_size = 4096,
        .input_data_size = 4096,
        .output_data_size = 4096,
        .host_exception_size = 4096};

    Handle mem_layout_ref = mem_layout_new(ctx, mem_cfg, code_size, stack_size, heap_size);
    handle_assert_no_error(ctx, mem_layout_ref);
    long guest_mem_size = mem_layout_get_memory_size(ctx, mem_layout_ref);
    Handle shared_mem_ref = shared_memory_new(ctx, guest_mem_size);
    handle_assert_no_error(ctx, shared_mem_ref);
    Handle mem_mgr_ref = mem_mgr_new(
        ctx,
        mem_cfg,
        shared_mem_ref,
        mem_layout_ref,
        true,
        100,
        guest_mem_size);
    handle_assert_no_error(ctx, mem_mgr_ref);
    uintptr_t address = shared_memory_get_address(ctx, shared_mem_ref);
    Handle offset_ref = mem_mgr_get_address_offset(ctx, mem_mgr_ref, address);
    handle_assert_no_error(ctx, offset_ref);
    uint64_t offset = handle_get_uint_64(ctx, offset_ref);
    Handle write_layout_result = mem_layout_write_memory_layout(ctx, mem_layout_ref, shared_mem_ref, address - offset, guest_mem_size);
    handle_assert_no_error(ctx, write_layout_result);

    // valid buffer

    unsigned char buffer[400] = {0x34, 0x01, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xf2, 0xfe, 0xff, 0xff, 0x04, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x08, 0x01, 0x00, 0x00, 0xdc, 0x00, 0x00, 0x00, 0xb0, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x68, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xd0, 0xff, 0xff, 0xff, 0x10, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x53, 0x74, 0x61, 0x74, 0x69, 0x63, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64, 0x57, 0x69, 0x74, 0x68, 0x41, 0x72, 0x67, 0x73, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0c, 0x00, 0x04, 0x00, 0x08, 0x00, 0x08, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x48, 0x6f, 0x73, 0x74, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64, 0x31, 0x00, 0x76, 0xff, 0xff, 0xff, 0x04, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x47, 0x65, 0x74, 0x54, 0x77, 0x6f, 0x00, 0x00, 0xb6, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01, 0x04, 0x00, 0x00, 0x00, 0x1b, 0x00, 0x00, 0x00, 0x47, 0x65, 0x74, 0x54, 0x69, 0x6d, 0x65, 0x53, 0x69, 0x6e, 0x63, 0x65, 0x42, 0x6f, 0x6f, 0x74, 0x4d, 0x69, 0x63, 0x72, 0x6f, 0x73, 0x65, 0x63, 0x6f, 0x6e, 0x64, 0x00, 0xe2, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01, 0x04, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x47, 0x65, 0x74, 0x54, 0x69, 0x63, 0x6b, 0x43, 0x6f, 0x75, 0x6e, 0x74, 0x00, 0x00, 0x0a, 0x00, 0x0c, 0x00, 0x08, 0x00, 0x00, 0x00, 0x07, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x04, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x47, 0x65, 0x74, 0x53, 0x74, 0x61, 0x63, 0x6b, 0x42, 0x6f, 0x75, 0x6e, 0x64, 0x61, 0x72, 0x79, 0x00, 0x00, 0x06, 0x00, 0x08, 0x00, 0x04, 0x00, 0x06, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x47, 0x65, 0x74, 0x4f, 0x53, 0x50, 0x61, 0x67, 0x65, 0x53, 0x69, 0x7a, 0x65, 0x00};

    Handle result = mem_mgr_write_host_function_details(ctx,
                                                        mem_mgr_ref,
                                                        buffer);

    handle_assert_no_error(ctx, result);
    handle_free(ctx, result);

    // invalid buffer

    unsigned char invalid_buffer[400] = {0x06, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0c, 0x00, 0x04, 0x00, 0x08, 0x00, 0x08, 0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0xd0, 0x00, 0x00, 0x00, 0xb0, 0x00, 0x00, 0x00, 0x84, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x54, 0xff, 0xff, 0xff,
                                         0x00, 0x00, 0x00, 0x04, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x08, 0x00, 0x07, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x8c, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x04, 0x08, 0x00, 0x00, 0x00, 0x04, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00, 0x00, 0xa0, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x03, 0x04, 0x00, 0x00, 0x00, 0x7a, 0xff, 0xff, 0xff, 0x04, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
                                         0x54, 0x65, 0x73, 0x74, 0x37, 0x00, 0x00, 0x00, 0xc0, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x03, 0x04, 0x00, 0x00, 0x00, 0x9a, 0xff, 0xff};

    result = mem_mgr_write_host_function_details(ctx,
                                                 mem_mgr_ref,
                                                 invalid_buffer);

    handle_assert_error(ctx, result);
    handle_free(ctx, result);

    // null_ptr

    unsigned char *null_ptr = 0;

    result = mem_mgr_write_host_function_details(ctx,
                                                 mem_mgr_ref,
                                                 null_ptr);

    handle_assert_error(ctx, result);
    handle_free(ctx, result);

    handle_free(ctx, write_layout_result);
    handle_free(ctx, offset_ref);
    handle_free(ctx, mem_layout_ref);
    handle_free(ctx, mem_mgr_ref);
    handle_free(ctx, shared_mem_ref);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_write_host_function_call()
{

    Context *ctx = context_new();
    size_t code_size = 4096;
    size_t stack_size = 4096;
    size_t heap_size = 4096;

    SandboxMemoryConfiguration mem_cfg = {
        .guest_error_buffer_size = 4096,
        .host_function_definition_size = 4096,
        .input_data_size = 4096,
        .output_data_size = 4096,
        .host_exception_size = 4096};

    Handle mem_layout_ref = mem_layout_new(ctx, mem_cfg, code_size, stack_size, heap_size);
    handle_assert_no_error(ctx, mem_layout_ref);
    long guest_mem_size = mem_layout_get_memory_size(ctx, mem_layout_ref);
    Handle shared_mem_ref = shared_memory_new(ctx, guest_mem_size);
    handle_assert_no_error(ctx, shared_mem_ref);
    Handle mem_mgr_ref = mem_mgr_new(
        ctx,
        mem_cfg,
        shared_mem_ref,
        mem_layout_ref,
        true,
        100,
        guest_mem_size);
    handle_assert_no_error(ctx, mem_mgr_ref);
    uintptr_t address = shared_memory_get_address(ctx, shared_mem_ref);
    Handle offset_ref = mem_mgr_get_address_offset(ctx, mem_mgr_ref, address);
    handle_assert_no_error(ctx, offset_ref);
    uint64_t offset = handle_get_uint_64(ctx, offset_ref);
    Handle write_layout_result = mem_layout_write_memory_layout(ctx, mem_layout_ref, shared_mem_ref, address - offset, guest_mem_size);
    handle_assert_no_error(ctx, write_layout_result);

    // valid buffer

    unsigned char buffer[152] = {0x94, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x10, 0x00, 0x08, 0x00, 0x0c, 0x00, 0x07, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x6c, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0e, 0x00, 0x07, 0x00, 0x08, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x08, 0x00, 0x04, 0x00, 0x06, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x66, 0x72, 0x6f, 0x6d, 0x20, 0x47, 0x75, 0x65, 0x73, 0x74, 0x46, 0x75, 0x6e, 0x63, 0x74, 0x69, 0x6f, 0x6e, 0x31, 0x2c, 0x20, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x66, 0x72, 0x6f, 0x6d, 0x20, 0x43, 0x61, 0x6c, 0x6c, 0x62, 0x61, 0x63, 0x6b, 0x54, 0x65, 0x73, 0x74, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x48, 0x6f, 0x73, 0x74, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64, 0x31, 0x00};

    Handle result = mem_mgr_write_host_function_call(ctx,
                                                     mem_mgr_ref,
                                                     buffer);

    handle_assert_no_error(ctx, result);
    handle_free(ctx, result);

#ifdef DEBUG

    // invalid buffer

    unsigned char invalid_buffer[400] = {0x2c, 0x01, 0x00, 0x00, 0x10, 0x00};

    result = mem_mgr_write_host_function_call(ctx,
                                              mem_mgr_ref,
                                              invalid_buffer);

    handle_assert_error(ctx, result);
    handle_free(ctx, result);

#endif

    // null_ptr

    unsigned char *null_ptr = 0;

    result = mem_mgr_write_host_function_call(ctx,
                                              mem_mgr_ref,
                                              null_ptr);

    handle_assert_error(ctx, result);
    handle_free(ctx, result);

    handle_free(ctx, write_layout_result);
    handle_free(ctx, offset_ref);
    handle_free(ctx, mem_layout_ref);
    handle_free(ctx, mem_mgr_ref);
    handle_free(ctx, shared_mem_ref);
    context_free(ctx);
    return MUNIT_OK;
}
