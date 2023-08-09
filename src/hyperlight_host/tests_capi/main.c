#include "munit/munit.h"
#include "sandbox_tests.h"
#include "context_tests.h"
#include "byte_array_tests.h"
#include "int.h"
#include "err.h"
#include "outb_handler.h"
#include "mem_access_handler.h"

static MunitSuite test_suites[] = {
    /* {name, tests, suites, iterations, options} */
    {"/byte_array_tests", byte_array_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/context_tests", context_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/err_tests", err_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/int_handle_tests", int_handle_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/mem_access_handler_tests", mem_access_handler_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/outb_handler_tests", outb_handler_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/sandbox_tests", sandbox_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
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
