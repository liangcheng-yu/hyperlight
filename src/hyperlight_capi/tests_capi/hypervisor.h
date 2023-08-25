#pragma once
#include <stdbool.h>

/**
 * This struct is setup in the test setup function hypervisor_set_availablity and is used to determine
 * If a Hypervisor should be available. 
 * It also indicates which Hypervisor is available if any.
 */
typedef struct  HypervisorAvailability {
  /**
   * If it is expected that hyperv on linux should be present
   */
  bool expect_hyperv_linux_present;

  /**
   * If it is expected that hyperv on linux should be a pre-release 
   */
  bool expect_hyperv_linux_prerelease_api;

  /**
   * If it is expected that KVM should be present
   */
  bool expect_kvm_present;

  /**
   * If it is expected that WHP should be present
   */
  bool expect_whp_present;

} HypervisorAvailabilityType;

void *hypervisor_check_flags(const MunitParameter params[], void *user_data);

bool check_kvm_available(HypervisorAvailabilityType *hypervisorAvailability);
bool check_hyperv_linux_available(HypervisorAvailabilityType *hypervisorAvailability);
bool check_whp_available(HypervisorAvailabilityType *hypervisorAvailability);

void hypervisor_check_flags_teardown(void *fixture);
