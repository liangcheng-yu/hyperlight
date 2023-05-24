#include "mem_mgr_tests.h"

struct Handle new_mem_mgr(struct Context *ctx)
{
    struct SandboxMemoryConfiguration mem_cfg = mem_config_default();
    Handle layout_ref = mem_layout_new(
        ctx,
        mem_cfg,
        0x100,
        0x100,
        0x1000);
    uint64_t mem_size = mem_layout_get_memory_size(ctx, layout_ref);
    Handle shared_mem_ref = shared_memory_new(ctx, mem_size);
    Handle mem_mgr_ref = mem_mgr_new(
        ctx,
        mem_cfg,
        shared_mem_ref,
        layout_ref,
        true,
        123,
        456);
    // free up the shared memory and layout Handles, because they've been
    // stored already inside the memory manager
    handle_free(ctx, shared_mem_ref);
    handle_free(ctx, layout_ref);
    return mem_mgr_ref;
}
