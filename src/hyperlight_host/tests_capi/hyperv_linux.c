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
#include "sandbox_tests.h"

MunitResult test_is_hyperv_linux_present(const MunitParameter params[], void *fixture)
{

    HypervisorAvailabilityType *hypervisorAvailability = (HypervisorAvailabilityType *)fixture;

    if (!hypervisorAvailability->expect_hyperv_linux_present) 
    {                                                                  
        return MUNIT_SKIP;                                             
    };

    // TODO: Handle pre release API properly
    // at present this test should succeed on hyperv linux with so long as the env var to expect a stable API is not set when running the test - (unless it is run on a machine with a stable API).
    munit_assert(check_hyperv_linux_available((HypervisorAvailabilityType*)fixture));
    return MUNIT_OK;
}

MunitResult test_hyperv_linux_create_driver(const MunitParameter params[], void *fixture)
{
    if (!check_hyperv_linux_available((HypervisorAvailabilityType*)fixture)) 
    {                                                                  
        return MUNIT_SKIP;                                             
    };

    const size_t MEM_SIZE = 0x1000;
    Context *ctx = context_new();
    Handle shared_mem_ref = shared_memory_new(ctx, MEM_SIZE);
    struct HypervisorAddrs addrs = {
        .entrypoint = 0,
        .guest_pfn = 0,
        .host_addr = shared_memory_get_address(ctx, shared_mem_ref),
        .mem_size = MEM_SIZE,
    };

    Handle hv_driver_hdl = hyperv_linux_create_driver(ctx, addrs, 0, 0);
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
