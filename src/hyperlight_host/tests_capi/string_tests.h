#include "munit/munit.h"
#include "hyperlight_host.h"

MunitResult test_string_create_free();

static MunitTest string_tests[] = {
    {
        "/test_string_create_free", /* name */
        test_string_create_free,    /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
