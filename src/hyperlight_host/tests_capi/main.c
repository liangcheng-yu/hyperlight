#include "munit/munit.h"
#include "sandbox_tests.h"
#include "val_ref_tests.h"
#include "pe_tests.h"
#include "context_tests.h"
#include "host_func_tests.h"
#include "byte_array_tests.h"
#include "mem_config.h"
#include "mem_layout.h"
#if defined(__linux__)
#include "hyperv_linux.h"
#endif
#include "int.h"
#include "guest_mem.h"
#include "munit/munit.h"

MunitTest sandbox_tests[] = {
    {
        "/test_is_hypervisor_present", /* name */
        test_is_hypervisor_present,    /* test */
        NULL,                          /* setup */
        NULL,                          /* tear_down */
        MUNIT_TEST_OPTION_NONE,        /* options */
        NULL                           /* parameters */
    },
    {
        "/test_get_binary_path",
        test_get_binary_path,
        NULL,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

MunitTest val_ref_tests[] = {
    {
        "/test_val_ref_new",    /* name */
        test_val_ref_new,       /* test */
        NULL,                   /* setup */
        NULL,                   /* tear_down */
        MUNIT_TEST_OPTION_NONE, /* options */
        NULL                    /* parameters */
    },
    {
        "/test_val_refs_compare",
        test_val_refs_compare,
        NULL,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

MunitTest host_func_tests[] = {
    {
        "/test_create_host_func", /* name */
        test_create_host_func,    /* test */
        NULL,                     /* setup */
        NULL,                     /* tear_down */
        MUNIT_TEST_OPTION_NONE,   /* options */
        NULL                      /* parameters */
    },
    {
        "/test_call_host_func",
        test_call_host_func,
        NULL,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

MunitTest context_tests[] = {
    {
        "/test_context_contains_memory", /* name */
        test_context_contains_memory,    /* test */
        NULL,                            /* setup */
        NULL,                            /* tear_down */
        MUNIT_TEST_OPTION_NONE,          /* options */
        NULL                             /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

MunitTest pe_file_tests[] = {
    {
        "/test_pe_relocate",    /* name */
        test_pe_relocate,       /* test */
        NULL,                   /* setup */
        NULL,                   /* tear_down */
        MUNIT_TEST_OPTION_NONE, /* options */
        NULL                    /* parameters */
    },
    {
        "/test_pe_getters",
        test_pe_getters,
        NULL,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

MunitTest byte_array_tests[] = {
    {
        "/test_byte_array_lifecycle", /* name */
        test_byte_array_lifecycle,    /* test */
        NULL,                         /* setup */
        NULL,                         /* tear_down */
        MUNIT_TEST_OPTION_NONE,       /* options */
        NULL                          /* parameters */
    },
    {
        "/test_byte_array_new_from_file",
        test_byte_array_new_from_file,
        NULL,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

MunitTest mem_config_tests[] = {
    {
        "/test_mem_config_getters", /* name */
        test_mem_config_getters,    /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

MunitTest mem_layout_tests[] = {
    {
        "/test_mem_layout_getters", /* name */
        test_mem_layout_getters,    /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

MunitTest int_handle_tests[] = {
    {
        "/test_int_64",             /* name */
        test_int_64,                /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {
        "/test_int_32",             /* name */
        test_int_32,                /* test */
        NULL,                       /* setup */
        NULL,                       /* tear_down */
        MUNIT_TEST_OPTION_NONE,     /* options */
        NULL                        /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

MunitTest guest_memory_tests[] = {
    {
        "/test_guest_mem_create_delete",/* name */
        test_guest_mem_create_delete,   /* test */
        NULL,                           /* setup */
        NULL,                           /* tear_down */
        MUNIT_TEST_OPTION_NONE,         /* options */
        NULL                            /* parameters */
    },
    {
        "/test_guest_mem_read_write",   /* name */
        test_guest_mem_read_write,      /* test */
        NULL,                           /* setup */
        NULL,                           /* tear_down */
        MUNIT_TEST_OPTION_NONE,         /* options */
        NULL                            /* parameters */
    },
    {
        "/test_guest_mem_copy_byte_array",  /* name */
        test_guest_mem_copy_byte_array,     /* test */
        NULL,                               /* setup */
        NULL,                               /* tear_down */
        MUNIT_TEST_OPTION_NONE,             /* options */
        NULL                                /* parameters */
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}};

#if defined(__linux__)
MunitTest hyperv_linux_tests[] = {
    {
        "/test_is_hyperv_linux_present", /* name */
        test_is_hyperv_linux_present,    /* test */
        set_flags,                            /* setup */
        NULL,                            /* tear_down */
        MUNIT_TEST_OPTION_NONE,          /* options */
        NULL                             /* parameters */
    },
    {
        "/test_open_mshv",
        test_open_mshv,
        set_flags,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {
        "/test_create_vm",
        test_create_vm,
        set_flags,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL},
    {
        "/test_create_vcpu",
        test_create_vcpu,
        set_flags,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {
        "/test_map_user_memory_region",
        test_map_user_memory_region,
        set_flags,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {
        "/test_set_registers",
        test_set_registers,
        set_flags,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {
        "/test_run_vcpu",
        test_run_vpcu,
        set_flags,
        NULL,
        MUNIT_TEST_OPTION_NONE,
        NULL
    },
    {NULL, NULL, NULL, NULL, MUNIT_TEST_OPTION_NONE, NULL}
};
#endif

static MunitSuite test_suites[] = {
    /* {name, tests, suites, iterations, options} */
    {"/sandbox_tests", sandbox_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/val_ref_tests", val_ref_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/host_func_tests", host_func_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/context_tests", context_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/pe_file_tests", pe_file_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/byte_array_tests", byte_array_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/mem_config_tests", mem_config_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/mem_layout_tests", mem_layout_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    #if defined(__linux__)
    {"/hyperv_linux_tests",hyperv_linux_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/int_handle_tests",int_handle_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    {"/guest_memory_tests",guest_memory_tests, NULL, 1, MUNIT_SUITE_OPTION_NONE},
    #endif
    
    {NULL, NULL, NULL, 0, MUNIT_SUITE_OPTION_NONE}
};
    

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