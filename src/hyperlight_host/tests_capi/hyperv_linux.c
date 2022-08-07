#if defined (__linux__)

#include "hyperv_linux.h"
#include "hyperlight_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <strings.h>
#include <sys/mman.h>
#include "err.h"
#include "munit/munit.h"

void set_flags()
{
    // Set env var HYPERV_SHOULD_BE_PRESENT to require hyperv to be present for this test.
    char* env_var = NULL;
    env_var = getenv("HYPERV_SHOULD_BE_PRESENT");
    munit_logf(MUNIT_LOG_INFO,"env var HYPERV_SHOULD_BE_PRESENT %s\n",env_var);

    if (env_var != NULL) {
        EXPECT_HYPERVISOR_PRESENT = get_flag_value(env_var);
    }
    
    // Set env var SHOULD_HAVE_STABLE_API to require a stable api for this test.
    env_var = NULL;
    env_var = getenv("SHOULD_HAVE_STABLE_API");
    munit_logf(MUNIT_LOG_INFO,"env var SHOULD_HAVE_STABLE_API %s\n",env_var);
    
    if (env_var != NULL) {
        EXPECT_PRERELEASE_API = !get_flag_value(env_var);
    }

    munit_logf(MUNIT_LOG_INFO,"EXPECT_HYPERVISOR_PRESENT: %s\n",EXPECT_HYPERVISOR_PRESENT ? "true" : "false");
    munit_logf(MUNIT_LOG_INFO,"EXPECT_PRERELEASE_API: %s\n",EXPECT_PRERELEASE_API ? "true" : "false");
}

bool get_flag_value(char* flag_value)
{
    if (strlen(flag_value) == 0)
    {
        return false;
    }
    
    if(strcasecmp(flag_value, "true") == 0 || strcasecmp(flag_value, "1") == 0 )
    {
        return true;
    }
    return false;
}

MunitResult test_is_hyperv_linux_present()
{
    bool status = is_hyperv_linux_present(false);
    if (EXPECT_HYPERVISOR_PRESENT && EXPECT_PRERELEASE_API)
    {
        munit_assert_true(status);
    }
    else
    {
        munit_assert_false(status);
    }

    status = is_hyperv_linux_present(true);
    if (EXPECT_HYPERVISOR_PRESENT && !EXPECT_PRERELEASE_API)
    {
        munit_assert_true(status);
    }
    else
    {
        munit_assert_false(status);
    }

    return MUNIT_OK;
}

MunitResult test_open_mshv()
{
    bool hypervisor_is_present = is_hyperv_linux_present(!EXPECT_PRERELEASE_API);

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle mshv = open_mshv(ctx, true);

    if (hypervisor_is_present && !EXPECT_PRERELEASE_API && EXPECT_HYPERVISOR_PRESENT)
    {
        handle_assert_no_error(ctx, mshv);
    }
    else
    {
         handle_assert_error(ctx, mshv);
    }

    handle_free(ctx, mshv);

    mshv = open_mshv(ctx, false);

    if (hypervisor_is_present && EXPECT_PRERELEASE_API && EXPECT_HYPERVISOR_PRESENT)
    {
        handle_assert_no_error(ctx, mshv);
    }
    else
    {
        handle_assert_error(ctx, mshv);
    }

    handle_free(ctx, mshv);
    context_free(ctx);
    return MUNIT_OK;

}

MunitResult test_create_vm()
{
    CHECK_IF_HYPERVISOR_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle mshv = open_mshv(ctx, !EXPECT_PRERELEASE_API);
    handle_assert_no_error(ctx, mshv);
    Handle vm = create_vm(ctx, mshv);
    handle_assert_no_error(ctx, vm);
    handle_free(ctx, vm);
    handle_free(ctx, mshv);
    context_free(ctx);
    
    return MUNIT_OK;
}

MunitResult test_create_vcpu()
{
    CHECK_IF_HYPERVISOR_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle mshv = open_mshv(ctx, !EXPECT_PRERELEASE_API);
    handle_assert_no_error(ctx, mshv);
    Handle vm = create_vm(ctx, mshv);
    handle_assert_no_error(ctx, vm);
    Handle vcpu = create_vcpu(ctx, vm);
    handle_assert_no_error(ctx, vcpu);
    handle_free(ctx, vcpu);
    handle_free(ctx, vm);
    handle_free(ctx, mshv);
    context_free(ctx);
    
    return MUNIT_OK;
}

MunitResult test_map_user_memory_region()
{
    CHECK_IF_HYPERVISOR_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle mshv = open_mshv(ctx, !EXPECT_PRERELEASE_API);
    handle_assert_no_error(ctx, mshv);
    Handle vm = create_vm(ctx, mshv);
    handle_assert_no_error(ctx, vm);
    uint64_t memSize = 0x1000;
    uint64_t guestPFN =  0x1;
    void *guestMemory = mmap(0, memSize, PROT_READ | PROT_WRITE, MAP_SHARED | 0x20 /* MAP-SHARED */ | 0x4000 /* MAP_NORESERVE */, -1, 0);
    munit_assert_not_null(guestMemory);
    Handle mshv_user_memory_region = map_vm_memory_region(ctx, vm, guestPFN, (uint64_t)guestMemory, memSize);
    handle_assert_no_error(ctx, mshv_user_memory_region);
    Handle should_be_empty = unmap_vm_memory_region(ctx, vm, mshv_user_memory_region);
    handle_assert_no_error(ctx, should_be_empty);
    handle_free(ctx, mshv_user_memory_region);
    handle_free(ctx, should_be_empty);
    
    munmap(guestMemory, memSize);

    handle_free(ctx, vm);
    handle_free(ctx, mshv);
    context_free(ctx);
    
    return MUNIT_OK;
}

MunitResult test_set_registers()
{
    CHECK_IF_HYPERVISOR_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle mshv = open_mshv(ctx, !EXPECT_PRERELEASE_API);
    handle_assert_no_error(ctx, mshv);
    Handle vm = create_vm(ctx, mshv);
    handle_assert_no_error(ctx, vm);
    Handle vcpu = create_vcpu(ctx, vm);
    handle_assert_no_error(ctx, vcpu);

    mshv_register mshvRegisters[] = {
    {.name = HV_X64_REGISTER_RBX, .reserved1=0, .reserved2=0, .value = {.low_part = 2, .high_part = 0}},
    {.name = HV_X64_REGISTER_RIP, .reserved1=0, .reserved2=0, .value = {.low_part = 0x1000, .high_part = 0}},
    {.name = HV_X64_REGISTER_RFLAGS, .reserved1=0, .reserved2=0, .value = {.low_part = 2, .high_part = 0}}};

    Handle result = set_registers(ctx, vcpu, mshvRegisters, 6);
    handle_assert_error(ctx, result);
    handle_free(ctx, result);

    result = set_registers(ctx, vcpu, mshvRegisters, 3);
    handle_assert_no_error(ctx, result);
    handle_free(ctx, result);

    handle_free(ctx, vcpu);
    handle_free(ctx, vm);
    handle_free(ctx, mshv);
    context_free(ctx);
    
    return MUNIT_OK;
}

MunitResult test_run_vpcu()
{

    CHECK_IF_HYPERVISOR_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle mshv = open_mshv(ctx, !EXPECT_PRERELEASE_API);
    handle_assert_no_error(ctx, mshv);
    Handle vm = create_vm(ctx, mshv);
    handle_assert_no_error(ctx, vm);
    Handle vcpu = create_vcpu(ctx, vm);
    handle_assert_no_error(ctx, vcpu);
    
    uint64_t memSize = 0x1000;
    uint64_t guestPFN =  0x1;
    void *guestMemory = mmap(0, memSize, PROT_READ | PROT_WRITE, MAP_SHARED | 0x20 /* MAP-SHARED */ | 0x4000 /* MAP_NORESERVE */, -1, 0);
    munit_assert_not_null(guestMemory);

    const uint8_t code[] = {
        0xba, 0xf8, 0x03, /* mov $0x3f8, %dx */
        0x00, 0xd8,       /* add %bl, %al */
        0x04, '0',        /* add $'0', %al */
        0xee,             /* out %al, (%dx) */
        /* send a 0 to indicate we're done */
        0xb0, '\0', /* mov $'\0', %al */
        0xee,
        0xf4, /* HLT */
    };

    memcpy((void*)guestMemory, code, sizeof(code));

    Handle mshv_user_memory_region = map_vm_memory_region(ctx, vm, guestPFN, (uint64_t)guestMemory, memSize);
    handle_assert_no_error(ctx, mshv_user_memory_region);
    
    mshv_register mshvRegisters[] = {
    {.name = HV_X64_REGISTER_CS, .value = {.low_part = 0, .high_part = 43628621390217215}},
    {.name = HV_X64_REGISTER_RAX, .value = {.low_part = 2, .high_part = 0}},
    {.name = HV_X64_REGISTER_RBX, .reserved1=0, .reserved2=0, .value = {.low_part = 2, .high_part = 0}},
    {.name = HV_X64_REGISTER_RIP, .reserved1=0, .reserved2=0, .value = {.low_part = 0x1000, .high_part = 0}},
    {.name = HV_X64_REGISTER_RFLAGS, .reserved1=0, .reserved2=0, .value = {.low_part = 2, .high_part = 0}}};

    Handle result = set_registers(ctx, vcpu, mshvRegisters, 5);
    handle_assert_no_error(ctx, result);
    handle_free(ctx, result);

    result = run_vcpu(ctx, vcpu);
    handle_assert_no_error(ctx, result);

    const mshv_run_message* run_message = get_run_result_from_handle(ctx, result);
    munit_assert_not_null(run_message);
    handle_free(ctx, result);

    munit_assert_uint32(run_message->message_type, ==, HV_MESSAGE_TYPE_HVMSG_X64_IO_PORT_INTERCEPT);
    munit_assert_uint64(run_message->rax, ==, (uint64_t)'4');
    munit_assert_uint16(run_message->port_number, ==, 0x3f8);
  
    mshv_register RIPReg[] = {
        {.name = HV_X64_REGISTER_RIP, .value = {.low_part = run_message->rip + run_message->instruction_length, .high_part = 0}}};

    free((void*)run_message);

    result = set_registers(ctx, vcpu, RIPReg, 1);
    handle_assert_no_error(ctx, result);
    handle_free(ctx, result);

    result = run_vcpu(ctx, vcpu);
    handle_assert_no_error(ctx, result);

    run_message = get_run_result_from_handle(ctx, result);
    munit_assert_not_null(run_message);
    handle_free(ctx, result);

    munit_assert_uint32(run_message->message_type, ==, HV_MESSAGE_TYPE_HVMSG_X64_IO_PORT_INTERCEPT);
    munit_assert_uint64(run_message->rax, ==, 0);
    munit_assert_uint16(run_message->port_number, ==, 0x3f8);

    mshv_register rip = {.name = HV_X64_REGISTER_RIP, .value = {.low_part = run_message->rip + run_message->instruction_length, .high_part = 0}};
    RIPReg[0] = rip;
    free((void*)run_message);

    result = set_registers(ctx, vcpu, RIPReg, 1);
    handle_assert_no_error(ctx, result);
    handle_free(ctx, result);
    
    result = run_vcpu(ctx, vcpu);
    handle_assert_no_error(ctx, result);

    run_message = get_run_result_from_handle(ctx, result);
    munit_assert_not_null(run_message);
    handle_free(ctx, result);

    munit_assert_uint32(run_message->message_type, ==, HV_MESSAGE_TYPE_HVMSG_X64_HALT);

    free((void*)run_message);

    Handle should_be_empty = unmap_vm_memory_region(ctx, vm, mshv_user_memory_region);
    handle_free(ctx, mshv_user_memory_region);
    handle_assert_no_error(ctx, should_be_empty);
    handle_free(ctx, should_be_empty);
    
    munmap(guestMemory, memSize);

    handle_free(ctx, vcpu);
    handle_free(ctx, vm);
    handle_free(ctx, mshv);
    context_free(ctx);
    
    return MUNIT_OK;
}
#endif
