#pragma once
#include "hyperlight_capi.h"
#include "munit/munit.h"

#define handle_assert_error(ctx, hdl) handle_assert_error_impl(ctx, hdl, __FILE__, __LINE__)
#define handle_assert_no_error(ctx, hdl) handle_assert_no_error_impl(ctx, hdl, __FILE__, __LINE__)

void handle_assert_error_impl(const Context *, Handle, const char *, int);
void handle_assert_no_error_impl(const Context *, Handle, const char *, int);

MunitResult test_handle_is_empty();
MunitResult test_handle_get_error_message();
MunitResult test_handle_new_error_null_ptr();

static MunitTest err_tests[] = {
    {
        "/test_handle_new_error_null_ptr", /* name */
        test_handle_new_error_null_ptr,    /* test */
        NULL,                              /* setup */
        NULL,                              /* tear_down */
        MUNIT_TEST_OPTION_NONE,            /* options */
        NULL                               /* parameters */
    },
    {
        "/test_handle_get_error_message", /* name */
        test_handle_get_error_message,    /* test */
        NULL,                             /* setup */
        NULL,                             /* tear_down */
        MUNIT_TEST_OPTION_NONE,           /* options */
        NULL                              /* parameters */
    },
    {
        "/test_handle_is_empty", /* name */
        test_handle_is_empty,    /* test */
        NULL,                    /* setup */
        NULL,                    /* tear_down */
        MUNIT_TEST_OPTION_NONE,  /* options */
        NULL                     /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
