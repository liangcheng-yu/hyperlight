#pragma once
#include "munit/munit.h"

MunitResult test_outb_handler_create(const MunitParameter[], void *);
MunitResult test_outb_handler_call(const MunitParameter[], void *);

static MunitTest outb_handler_tests[] = {
    {
        (char *)"/test_outb_handler_create", /* name */
        test_outb_handler_create,            /* test */
        NULL,                                /* setup */
        NULL,                                /* tear_down */
        MUNIT_TEST_OPTION_NONE,              /* options */
        NULL                                 /* parameters */
    },
    {
        (char *)"/test_outb_handler_call", /* name */
        test_outb_handler_call,            /* test */
        NULL,                              /* setup */
        NULL,                              /* tear_down */
        MUNIT_TEST_OPTION_NONE,            /* options */
        NULL                               /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
