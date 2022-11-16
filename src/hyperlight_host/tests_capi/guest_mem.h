#pragma once
#include "munit/munit.h"
#include "hyperlight_host.h"

MunitResult test_guest_mem_copy_from_byte_array();
MunitResult test_guest_mem_copy_to_byte_array();
MunitResult test_guest_mem_create_delete();
MunitResult test_guest_mem_read_write();

static MunitTest guest_memory_tests[] = {
    {
        "/test_guest_mem_create_delete", /* name */
        test_guest_mem_create_delete,    /* test */
        NULL,                            /* setup */
        NULL,                            /* tear_down */
        MUNIT_TEST_OPTION_NONE,          /* options */
        NULL                             /* parameters */
    },
    {
        "/test_guest_mem_read_write", /* name */
        test_guest_mem_read_write,    /* test */
        NULL,                         /* setup */
        NULL,                         /* tear_down */
        MUNIT_TEST_OPTION_NONE,       /* options */
        NULL                          /* parameters */
    },
    {
        "/test_guest_mem_copy_from_byte_array", /* name */
        test_guest_mem_copy_from_byte_array,    /* test */
        NULL,                                   /* setup */
        NULL,                                   /* tear_down */
        MUNIT_TEST_OPTION_NONE,                 /* options */
        NULL                                    /* parameters */
    },
    {
        "/test_guest_mem_copy_to_byte_array", /* name */
        test_guest_mem_copy_to_byte_array,    /* test */
        NULL,                                 /* setup */
        NULL,                                 /* tear_down */
        MUNIT_TEST_OPTION_NONE,               /* options */
        NULL                                  /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};
