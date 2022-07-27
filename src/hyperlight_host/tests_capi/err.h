#pragma once
#include "hyperlight_host.h"

#define handle_assert_error(ctx, hdl) handle_assert_error_impl(ctx, hdl, __FILE__, __LINE__)
#define handle_assert_no_error(ctx, hdl) handle_assert_no_error_impl(ctx, hdl, __FILE__, __LINE__)

void handle_assert_error_impl(const Context *, Handle, const char *, int);
void handle_assert_no_error_impl(const Context *, Handle, const char *, int);
