#include "hyperlight_host.h"
#include "munit/munit.h"
#include "val_ref.h"
#include "callback.h"
#include "host_exception.h"
#include "err.h"
#include "stdio.h"

static const size_t CODE_SIZE = 0x1000;
static const size_t STACK_SIZE = 0x1000;
static const size_t HEAP_SIZE = 0x1000;
static SandboxMemoryConfiguration mem_cfg = {
        .guest_error_message_size = 0x20,
        .host_function_definition_size = 0x100,
        .input_data_size = 0x100,
        .output_data_size = 0x100,
        .host_exception_size = 0x20};

Handle mem_layout_ref;
Handle mem_mgr_ref;
long guest_mem_size = 0;
Handle guest_mem_ref;

void setup_memory(Context *ctx)
{
    mem_layout_ref = mem_layout_new(ctx, mem_cfg, CODE_SIZE, STACK_SIZE, HEAP_SIZE);
    handle_assert_no_error(ctx,mem_layout_ref);
    mem_mgr_ref = mem_mgr_new(ctx, mem_cfg, true);
    handle_assert_no_error(ctx,mem_mgr_ref);
    guest_mem_size = mem_layout_get_memory_size(ctx,mem_layout_ref);
    guest_mem_ref = guest_memory_new(ctx, guest_mem_size);
    handle_assert_no_error(ctx,guest_mem_ref);
    uintptr_t address = guest_memory_get_address(ctx, guest_mem_ref);
    Handle offset_ref = mem_mgr_get_address_offset(ctx, mem_mgr_ref, address);
    handle_assert_no_error(ctx,offset_ref);
    uint64_t offset = handle_get_uint_64(ctx, offset_ref);
    Handle write_layout_result =  mem_layout_write_memory_layout(ctx, mem_layout_ref, guest_mem_ref, address-offset, guest_mem_size);
    handle_assert_no_error(ctx,write_layout_result);
    handle_free(ctx, offset_ref);
    handle_free(ctx, write_layout_result);
}

MunitResult test_has_host_exception()
{
    Context *ctx = context_new();
    setup_memory(ctx);

    Handle host_exception_ref = mem_mgr_has_host_exception(ctx,mem_mgr_ref, mem_layout_ref, guest_mem_ref);
    handle_assert_no_error(ctx, host_exception_ref);     
    bool has_host_exception = handle_get_boolean(ctx, host_exception_ref);
    munit_assert_false(has_host_exception); 
    handle_free(ctx, host_exception_ref);                            

    const char *err_msg = "test error message";
    Handle byte_array_1_ref = byte_array_new(ctx,(const uint8_t *)err_msg,strlen(err_msg));
    handle_assert_no_error(ctx, byte_array_1_ref);

    const char *exception_data = "test exception data";
    Handle byte_array_2_ref = byte_array_new(ctx,(const uint8_t *)exception_data,strlen(exception_data));
    handle_assert_no_error(ctx, byte_array_2_ref);

    Handle result_ref = mem_mgr_write_outb_exception(ctx, mem_mgr_ref, mem_layout_ref, guest_mem_ref, byte_array_1_ref, byte_array_2_ref);
    handle_assert_no_error(ctx,result_ref);

    host_exception_ref = mem_mgr_has_host_exception(ctx,mem_mgr_ref, mem_layout_ref, guest_mem_ref);
    handle_assert_no_error(ctx, host_exception_ref);     
    has_host_exception = handle_get_boolean(ctx, host_exception_ref);
    munit_assert_true(has_host_exception); 
    handle_free(ctx, host_exception_ref);     
    
    guest_mem_size=0;
    handle_free(ctx, result_ref);
    handle_free(ctx, byte_array_2_ref);
    handle_free(ctx, byte_array_1_ref);
    handle_free(ctx, mem_mgr_ref);
    handle_free(ctx, guest_mem_ref);
    handle_free(ctx, mem_layout_ref);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_host_exception_length()
{

    Context *ctx = context_new();
    setup_memory(ctx);

    Handle host_exception_length_ref = mem_mgr_get_host_exception_length(ctx,mem_mgr_ref, mem_layout_ref, guest_mem_ref);
    handle_assert_no_error(ctx, host_exception_length_ref);     
    int32_t host_exception_length = handle_get_int_32(ctx, host_exception_length_ref);
    munit_assert_int32(host_exception_length, ==, 0);
    handle_free(ctx, host_exception_length_ref);                            

    const char *err_msg = "test error message";
    Handle byte_array_1_ref = byte_array_new(ctx,(const uint8_t *)err_msg,strlen(err_msg));
    handle_assert_no_error(ctx, byte_array_1_ref);

    const char *exception_data = "test exception data";
    Handle byte_array_2_ref = byte_array_new(ctx,(const uint8_t *)exception_data,strlen(exception_data));
    handle_assert_no_error(ctx, byte_array_2_ref);

    Handle result_ref = mem_mgr_write_outb_exception(ctx, mem_mgr_ref, mem_layout_ref, guest_mem_ref, byte_array_1_ref, byte_array_2_ref);
    handle_assert_no_error(ctx,result_ref);

    host_exception_length_ref = mem_mgr_get_host_exception_length(ctx,mem_mgr_ref, mem_layout_ref, guest_mem_ref);
    handle_assert_no_error(ctx, host_exception_length_ref);     
    host_exception_length = handle_get_int_32(ctx, host_exception_length_ref);
    munit_assert_int32(host_exception_length, ==, strlen(exception_data));
    handle_free(ctx, host_exception_length_ref);              
    
    guest_mem_size=0;
    handle_free(ctx, result_ref);
    handle_free(ctx, byte_array_2_ref);
    handle_free(ctx, byte_array_1_ref);
    handle_free(ctx, mem_mgr_ref);
    handle_free(ctx, guest_mem_ref);
    handle_free(ctx, mem_layout_ref);
    context_free(ctx);
    return MUNIT_OK;
}


MunitResult test_long_data_causes_errors()
{

    Context *ctx = context_new();
    setup_memory(ctx);

    const char *err_msg = "test error message that should be much too long to handle";
    Handle byte_array_1_ref = byte_array_new(ctx,(const uint8_t *)err_msg,strlen(err_msg));
    handle_assert_no_error(ctx, byte_array_1_ref);

    const char *exception_data = "test exception data";
    Handle byte_array_2_ref = byte_array_new(ctx,(const uint8_t *)exception_data,strlen(exception_data));
    handle_assert_no_error(ctx, byte_array_2_ref);

    Handle result_ref = mem_mgr_write_outb_exception(ctx, mem_mgr_ref, mem_layout_ref, guest_mem_ref, byte_array_1_ref, byte_array_2_ref);
    handle_assert_error(ctx,result_ref);
 
    handle_free(ctx, byte_array_1_ref);
    handle_free(ctx, byte_array_2_ref);
    handle_free(ctx, result_ref);
    err_msg = "test error message";
    byte_array_1_ref = byte_array_new(ctx,(const uint8_t *)err_msg,strlen(err_msg));
    handle_assert_no_error(ctx, byte_array_1_ref);

    exception_data = "test exception data that should be much too long to handle";
    byte_array_2_ref = byte_array_new(ctx,(const uint8_t *)exception_data,strlen(exception_data));
    handle_assert_no_error(ctx, byte_array_2_ref);

    result_ref = mem_mgr_write_outb_exception(ctx, mem_mgr_ref, mem_layout_ref, guest_mem_ref, byte_array_1_ref, byte_array_2_ref);
    handle_assert_error(ctx,result_ref);

    guest_mem_size=0;
    handle_free(ctx, result_ref);
    handle_free(ctx, byte_array_2_ref);
    handle_free(ctx, byte_array_1_ref);
    handle_free(ctx, mem_mgr_ref);
    handle_free(ctx, guest_mem_ref);
    handle_free(ctx, mem_layout_ref);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_host_exception_data_round_trip()
{

    Context *ctx = context_new();
    setup_memory(ctx);

    const char *err_msg = "test error message";
    Handle byte_array_1_ref = byte_array_new(ctx,(const uint8_t *)err_msg,strlen(err_msg));
    handle_assert_no_error(ctx, byte_array_1_ref);

    const char *exception_data = "test exception data";
    Handle byte_array_2_ref = byte_array_new(ctx,(const uint8_t *)exception_data,strlen(exception_data));
    handle_assert_no_error(ctx, byte_array_2_ref);

    Handle result_ref = mem_mgr_write_outb_exception(ctx, mem_mgr_ref, mem_layout_ref, guest_mem_ref, byte_array_1_ref, byte_array_2_ref);
    handle_assert_no_error(ctx,result_ref);      

    Handle host_exception_length_ref = mem_mgr_get_host_exception_length(ctx,mem_mgr_ref, mem_layout_ref, guest_mem_ref);
    handle_assert_no_error(ctx, host_exception_length_ref);     
    int32_t host_exception_length = handle_get_int_32(ctx, host_exception_length_ref);

    munit_assert_int32(host_exception_length, ==, strlen(exception_data));

    unsigned char* exception_data1 = (unsigned char *)malloc(host_exception_length+1);
    memset(exception_data1, 0, host_exception_length+1);
    
    result_ref = mem_mgr_get_host_exception_data(ctx, mem_mgr_ref, mem_layout_ref, guest_mem_ref, exception_data1, host_exception_length);
    handle_assert_no_error(ctx, result_ref);     

    munit_assert_string_equal((const char *)exception_data1, exception_data);
    
    free((void *)exception_data1);
    guest_mem_size=0;
    handle_free(ctx, host_exception_length_ref);     
    handle_free(ctx, result_ref);
    handle_free(ctx, byte_array_2_ref);
    handle_free(ctx, byte_array_1_ref);
    handle_free(ctx, mem_mgr_ref);
    handle_free(ctx, guest_mem_ref);
    handle_free(ctx, mem_layout_ref);
    context_free(ctx);
    return MUNIT_OK;

}