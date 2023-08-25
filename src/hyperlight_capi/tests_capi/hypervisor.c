#include "munit/munit.h"
#include <stdbool.h>
#include "flag.h"
#include "hypervisor.h"
#include "hyperlight_capi.h"

bool check_only_one_set(bool a, bool b, bool c)
{
    int count = 0;
    if (a)
    {
        count++;
    }

    if (b)
    {
        count++;
    }

    if (c)
    {
        count++;
    }

    return count < 2;
}

// TODO: These definitions should be moved to sandbox_tests.h once we have a C API for the sandbox.
// They will be used by the sandbox tests at that point and we should not have any C Tests for Hypervisors, they should be Rust only.
// TODO: Add WHP Support

void *hypervisor_check_flags(const MunitParameter params[], void *user_data)
{

    HypervisorAvailabilityType *hypervisorAvailability =  (HypervisorAvailabilityType*)malloc(sizeof(HypervisorAvailabilityType));

    hypervisorAvailability->expect_hyperv_linux_present = false;
    hypervisorAvailability->expect_hyperv_linux_prerelease_api = true;
    hypervisorAvailability->expect_kvm_present = false;
    hypervisorAvailability->expect_whp_present = false;

#ifdef __linux__

    // Set env var HYPERV_SHOULD_BE_PRESENT to true to require hyperv to be present for this test.
    char *env_var = NULL;
    env_var = getenv("HYPERV_SHOULD_BE_PRESENT");
    munit_logf(MUNIT_LOG_INFO, "env var HYPERV_SHOULD_BE_PRESENT %s\n", env_var);

    if (env_var != NULL)
    {
        hypervisorAvailability->expect_hyperv_linux_present = get_flag_value(env_var);
    }

    // Set env var HYPERV_SHOULD_HAVE_STABLE_API to false to require a stable api for this test.
    env_var = NULL;
    env_var = getenv("HYPERV_SHOULD_HAVE_STABLE_API");
    munit_logf(MUNIT_LOG_INFO, "env var HYPERV_SHOULD_HAVE_STABLE_API %s\n", env_var);

    if (env_var != NULL)
    {
        hypervisorAvailability->expect_hyperv_linux_prerelease_api = !get_flag_value(env_var);
    }

    // Set env var KVM_SHOULD_BE_PRESENT to true to require KVM for this test.
    env_var = NULL;
    env_var = getenv("KVM_SHOULD_BE_PRESENT");
    munit_logf(MUNIT_LOG_INFO, "env var KVM_SHOULD_BE_PRESENT %s\n", env_var);

    if (env_var != NULL)
    {
        hypervisorAvailability->expect_kvm_present = get_flag_value(env_var);
    }

#endif

#ifdef _WIN32

    // Set env var WHP_SHOULD_BE_PRESENT to true to require WHP to be present for this test.
    char * env_var_buffer = NULL;
    size_t env_var_buffer_size = 0;
    _dupenv_s(&env_var_buffer, &env_var_buffer_size, "WHP_SHOULD_BE_PRESENT");
    munit_logf(MUNIT_LOG_INFO, "env var WHP_SHOULD_BE_PRESENT %s\n", env_var_buffer);

    if (env_var_buffer != NULL)
    {
        hypervisorAvailability->expect_whp_present = get_flag_value(env_var_buffer);
    }

#endif

    munit_logf(MUNIT_LOG_INFO, "EXPECT_HYPERV_LINUX_PRESENT: %s\n", hypervisorAvailability->expect_hyperv_linux_present ? "true" : "false");
    munit_logf(MUNIT_LOG_INFO, "EXPECT_HYPERV_LINUX_PRERELEASE_API: %s\n", hypervisorAvailability->expect_hyperv_linux_prerelease_api ? "true" : "false");
    munit_logf(MUNIT_LOG_INFO, "EXPECT_KVM_PRESENT: %s\n", hypervisorAvailability->expect_kvm_present ? "true" : "false");
    munit_logf(MUNIT_LOG_INFO, "EXPECT_WHP_PRESENT: %s\n", hypervisorAvailability->expect_whp_present ? "true" : "false");
    
    if (!check_only_one_set(hypervisorAvailability->expect_kvm_present, hypervisorAvailability->expect_hyperv_linux_present, hypervisorAvailability->expect_whp_present))
    {
        munit_log(MUNIT_LOG_INFO, "Only one of KVM_SHOULD_BE_PRESENT, WHP_SHOULD_BE_PRESENT and HYPERV_SHOULD_BE_PRESENT should be set.\n");
        exit(1);
    }
    
    return (void*) hypervisorAvailability;

}

bool check_kvm_available(HypervisorAvailabilityType *hypervisorAvailability)
{
    
    if (is_hypervisor_present() && hypervisorAvailability->expect_kvm_present)
    {
        return true;
    }

    return false;
}

bool check_hyperv_linux_available(HypervisorAvailabilityType *hypervisorAvailability)
{
    if (is_hypervisor_present() && hypervisorAvailability->expect_hyperv_linux_present)
    {
        return true;
    }

    return false;
}

bool check_whp_available(HypervisorAvailabilityType *hypervisorAvailability)
{
    if (is_hypervisor_present() && hypervisorAvailability->expect_whp_present)
    {
        return true;
    }

    return false;
}

void hypervisor_check_flags_teardown(void *fixture)
{
    free(fixture);
}