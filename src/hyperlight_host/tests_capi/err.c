#include "err.h"
#include "hyperlight_host.h"
#include "stdio.h"
#include "stdlib.h"

void handle_assert_error_impl(const Context *ctx, Handle hdl, const char *file, int line)
{
    if (!handle_is_error(hdl))
    {
        fprintf(stderr, "[%s:%d] expected error but got none\n", file, line);
        exit(1);
    }
}

void handle_assert_no_error_impl(const Context *ctx, Handle h, const char *file, int line)
{
    if (handle_is_error(h))
    {
        const char *err_msg = handle_get_error_message(ctx, h);
        fprintf(stderr, "[%s:%d] handle error: %s\n", file, line, err_msg);
        free((char *)err_msg);
        exit(1);
    }
}
