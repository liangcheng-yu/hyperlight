#include "munit/munit.h"
#include "sandbox_tests.h"
#include "val_ref_tests.h"
#include "context_tests.h"
#include "host_func_tests.h"
#include "host_exception.h"
#include "byte_array_tests.h"
#include "mem_layout.h"
#if defined(__linux__)
#include "hyperv_linux.h"
#endif
#include "int.h"
#include "shared_mem.h"
#include "err.h"
#include "outb_handler.h"
#include "mem_access_handler.h"
#include "string_tests.h"
#include "guest_func_tests.h"

static MunitSuite test_suites[] = {
    /* {name, tests, suites, iterations, options} */
    {"/byte_array_tests", byte_array_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/context_tests", context_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/err_tests", err_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/shared_memory_tests", shared_memory_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/host_exception_tests", host_exception_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/host_func_tests", host_func_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
#if defined(__linux__)
    {"/hyperv_linux_tests", hyperv_linux_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
#endif
    {"/int_handle_tests", int_handle_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/mem_access_handler_tests", mem_access_handler_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/mem_layout_tests", mem_layout_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/outb_handler_tests", outb_handler_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/sandbox_tests", sandbox_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/val_ref_tests", val_ref_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/string_tests", string_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/guest_func_tests", guest_func_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {NULL, NULL, NULL, 0, MUNIT_SUITE_OPTION_NONE}};

static const MunitSuite sandbox_test = {
    "/tests_capi",          /* name */
    NULL,                   /* tests */
    test_suites,            /* suites */
    1,                      /* iterations */
    MUNIT_SUITE_OPTION_NONE /* options */
};

int main(int argc, char *argv[MUNIT_ARRAY_PARAM(argc + 1)])
{
    return munit_suite_main(&sandbox_test, NULL, argc, argv);
}
