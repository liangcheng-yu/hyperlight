#if defined(__linux__)

#include "kvm.h"
#include "hyperlight_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <strings.h>
#include <sys/mman.h>
#include "err.h"
#include "munit/munit.h"
#include "flag.h"

void *kvm_set_flags(const MunitParameter params[], void *user_data)
{
    // Set env var KVM_SHOULD_BE_PRESENT to require KVM to be present
    // for this test.
    char *env_var = NULL;
    env_var = getenv("KVM_SHOULD_BE_PRESENT");
    munit_logf(MUNIT_LOG_INFO, "env var KVM_SHOULD_BE_PRESENT %s\n", env_var);

    if (env_var != NULL)
    {
        EXPECT_KVM_PRESENT = get_flag_value(env_var);
    }

    munit_logf(MUNIT_LOG_INFO, "EXPECT_KVM_PRESENT: %s\n", EXPECT_KVM_PRESENT ? "true" : "false");
    return NULL;
}

MunitResult test_is_kvm_present(const MunitParameter params[], void *fixture)
{
    bool status = kvm_is_present();
    if (EXPECT_KVM_PRESENT)
    {
        munit_assert_true(status);
    }
    else
    {
        munit_assert_false(status);
    }

    return MUNIT_OK;
}

MunitResult test_kvm_open(const MunitParameter params[], void *fixture)
{
    bool present = kvm_is_present();

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle kvm = kvm_open(ctx);

    if (present && EXPECT_KVM_PRESENT)
    {
        handle_assert_no_error(ctx, kvm);
    }
    else
    {
        handle_assert_error(ctx, kvm);
    }

    handle_free(ctx, kvm);
    context_free(ctx);
    return MUNIT_OK;
}

MunitResult test_kvm_create_vm(const MunitParameter params[], void *fixture)
{
    CHECK_KVM_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle kvm = kvm_open(ctx);
    handle_assert_no_error(ctx, kvm);
    Handle vm = kvm_create_vm(ctx, kvm);
    handle_assert_no_error(ctx, vm);
    handle_free(ctx, vm);
    handle_free(ctx, kvm);
    context_free(ctx);

    return MUNIT_OK;
}

MunitResult test_kvm_create_vcpu(const MunitParameter params[], void *fixture)
{
    CHECK_KVM_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle kvm = kvm_open(ctx);
    handle_assert_no_error(ctx, kvm);
    Handle vm = kvm_create_vm(ctx, kvm);
    handle_assert_no_error(ctx, vm);
    Handle vcpu = kvm_create_vcpu(ctx, vm);
    handle_assert_no_error(ctx, vcpu);
    handle_free(ctx, vcpu);
    handle_free(ctx, vm);
    handle_free(ctx, kvm);
    context_free(ctx);

    return MUNIT_OK;
}

MunitResult test_kvm_map_user_memory_region(const MunitParameter params[], void *fixture)
{
    CHECK_KVM_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle kvm = kvm_open(ctx);
    handle_assert_no_error(ctx, kvm);
    Handle vm = kvm_create_vm(ctx, kvm);
    handle_assert_no_error(ctx, vm);
    uint64_t memSize = 0x1000;
    void *guestMemory = mmap(
        0,
        memSize,
        PROT_READ | PROT_WRITE,
        (
            MAP_SHARED |
            0x20 /* MAP-SHARED */ |
            0x4000 /* MAP_NORESERVE */
            ),
        -1,
        0);
    munit_assert_not_null(guestMemory);
    Handle kvm_user_memory_region = kvm_map_vm_memory_region(
        ctx,         // context
        vm,          // vm handle
        0x0,         // guest physical address loads at 0
        guestMemory, // pointer to the guest memory
        memSize);    // the size of the guest memory
    handle_assert_no_error(ctx, kvm_user_memory_region);
    Handle should_be_empty = kvm_unmap_vm_memory_region(ctx, vm, kvm_user_memory_region);
    handle_assert_no_error(ctx, should_be_empty);
    handle_free(ctx, should_be_empty);

    handle_free(ctx, kvm_user_memory_region);
    munmap(guestMemory, memSize);

    handle_free(ctx, vm);
    handle_free(ctx, kvm);
    context_free(ctx);

    return MUNIT_OK;
}

MunitResult test_kvm_set_registers(const MunitParameter params[], void *fixture)
{
    CHECK_KVM_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle kvm = kvm_open(ctx);
    handle_assert_no_error(ctx, kvm);
    Handle vm = kvm_create_vm(ctx, kvm);
    handle_assert_no_error(ctx, vm);
    Handle vcpu = kvm_create_vcpu(ctx, vm);
    handle_assert_no_error(ctx, vcpu);

    Regs kvmRegisters = {
        .rbx = 2,
        .rip = 0x1000,
        .rflags = 2};

    Handle result = kvm_set_registers(ctx, vcpu, kvmRegisters);
    handle_assert_no_error(ctx, result);
    handle_free(ctx, result);

    handle_free(ctx, vcpu);
    handle_free(ctx, vm);
    handle_free(ctx, kvm);
    context_free(ctx);

    return MUNIT_OK;
}

MunitResult test_kvm_run_vpcu(const MunitParameter params[], void *fixture)
{
    CHECK_KVM_PRESENT;

    Context *ctx = context_new();
    munit_assert_not_null(ctx);
    Handle kvm = kvm_open(ctx);
    handle_assert_no_error(ctx, kvm);
    Handle vm = kvm_create_vm(ctx, kvm);
    handle_assert_no_error(ctx, vm);
    Handle vcpu = kvm_create_vcpu(ctx, vm);
    handle_assert_no_error(ctx, vcpu);

    uint64_t memSize = 0x1000;
    uint64_t guestPhysAddr = 0x1000;
    void *guestMemory = mmap(
        0,
        memSize,
        PROT_READ | PROT_WRITE,
        (
            MAP_SHARED |
            0x20 /* MAP-SHARED */ |
            0x4000 /* MAP_NORESERVE */
            ),
        -1,
        0);
    munit_assert_not_null(guestMemory);

    const uint8_t code[] = {
        0xba, 0xf8, 0x03, /* mov $0x3f8, %dx */
        0x00, 0xd8,       /* add %bl, %al */
        0x04, '0',        /* add $'0', %al */
        0xee,             /* out %al, (%dx) */
        0xb0, '\0',       /* mov $'\n', %al */
        0xee,             /* out %al, (%dx) */
        0xf4,             /* hlt */
    };

    memcpy((void *)guestMemory, code, sizeof(code));

    Handle kvm_user_memory_region = kvm_map_vm_memory_region(
        ctx,
        vm,
        guestPhysAddr,
        guestMemory,
        memSize);
    handle_assert_no_error(ctx, kvm_user_memory_region);

    Regs regs = {
        .rip = 0x1000,
        .rax = 2,
        .rbx = 2,
        .rflags = 0x2};

    Handle set_reg_res = kvm_set_registers(ctx, vcpu, regs);
    handle_assert_no_error(ctx, set_reg_res);
    handle_free(ctx, set_reg_res);

    Handle sregs_ref = kvm_get_sregisters(ctx, vcpu);
    handle_assert_no_error(ctx, sregs_ref);
    CSRegs *csregs = kvm_get_sregisters_from_handle(ctx, sregs_ref);
    munit_assert_not_null(csregs);
    csregs->cs.base = 0;
    csregs->cs.selector = 0;
    Handle set_sreg_res = kvm_set_sregisters(ctx, vcpu, sregs_ref, *csregs);
    handle_assert_no_error(ctx, set_sreg_res);
    free(csregs);
    handle_free(ctx, sregs_ref);
    handle_free(ctx, set_sreg_res);

    {
        Handle run_res = kvm_run_vcpu(ctx, vcpu);
        handle_assert_no_error(ctx, run_res);

        const KvmRunMessage *run_message = kvm_get_run_result_from_handle(ctx, run_res);
        munit_assert_not_null(run_message);
        handle_free(ctx, run_res);

        // the code does two addition operations (values of rbx + rax registers)
        // and sends each result via an IoOut
        munit_assert_uint32(run_message->message_type, ==, IOOut);
        munit_assert_uint64(run_message->rax, ==, (uint64_t)'4');
        munit_assert_uint16(run_message->port_number, ==, 0x3f8);

        Handle regs_ref = kvm_get_registers(ctx, vcpu);
        handle_assert_no_error(ctx, regs_ref);
        Regs *regs_after = kvm_get_registers_from_handle(ctx, regs_ref);
        munit_assert_not_null(regs_after);
        munit_assert_uint16(run_message->rip, ==, regs_after->rip);
        free(regs_after);
        handle_free(ctx, regs_ref);

        kvm_free_run_result((struct KvmRunMessage *)run_message);
    }

    // result = set_registers(ctx, vcpu, RIPReg, 1);
    // handle_assert_no_error(ctx, result);
    // handle_free(ctx, result);

    // NOTE: KVM automatically advances to the next instruction.
    // we do not need to manually advance.
    {
        Handle run_res = kvm_run_vcpu(ctx, vcpu);
        handle_assert_no_error(ctx, run_res);

        const KvmRunMessage *run_message = kvm_get_run_result_from_handle(ctx, run_res);
        munit_assert_not_null(run_message);
        handle_free(ctx, run_res);

        munit_assert_uint32(run_message->message_type, ==, IOOut);
        munit_assert_uint64(run_message->rax, ==, 0);
        munit_assert_uint16(run_message->port_number, ==, 0x3f8);
        kvm_free_run_result((struct KvmRunMessage *)run_message);
        handle_free(ctx, run_res);
    }
    {
        Handle run_res = kvm_run_vcpu(ctx, vcpu);
        handle_assert_no_error(ctx, run_res);

        const KvmRunMessage *run_message = kvm_get_run_result_from_handle(ctx, run_res);
        munit_assert_not_null(run_message);
        handle_free(ctx, run_res);

        munit_assert_uint32(run_message->message_type, ==, Halt);
        kvm_free_run_result((struct KvmRunMessage *)run_message);
    }

    Handle should_be_empty = kvm_unmap_vm_memory_region(
        ctx,
        vm,
        kvm_user_memory_region);
    handle_assert_no_error(ctx, should_be_empty);

    handle_free(ctx, should_be_empty);
    handle_free(ctx, kvm_user_memory_region);
    munmap(guestMemory, memSize);

    handle_free(ctx, vcpu);
    handle_free(ctx, vm);
    handle_free(ctx, kvm);
    context_free(ctx);

    return MUNIT_OK;
}
#endif
