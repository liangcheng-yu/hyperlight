use seccompiler::SeccompCmpOp::Eq;
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen as ArgLen, SeccompCondition as Cond, SeccompFilter,
    SeccompRule,
};

use crate::sandbox::hypervisor::{HypervisorType, AVAILABLE_HYPERVISOR};
use crate::HyperlightError::NoHypervisorFound;
use crate::{and, or, Result};

const TCGETS: u64 = 0x5401;

mod mshv {
    pub const MSHV_UNMAP_GUEST_MEMORY: u64 = 0x4020_b803;
    pub const MSHV_GET_VP_REGISTERS: u64 = 0xc010_b805;
    pub const MSHV_SET_VP_REGISTERS: u64 = 0x4010_b806;
    pub const MSHV_RUN_VP: u64 = 0x8100_b807;
    pub const MSHV_GET_VP_STATE: u64 = 0xc010_b80a;
    pub const MSHV_ROOT_HVCALL: u64 = 0xc020_b835;
}

mod kvm {
    pub const KVM_SET_REGS: u64 = 0x4090_ae82;
    pub const KVM_SET_FPU: u64 = 0x41a0_ae8d;
    pub const KVM_RUN: u64 = 0xae80;
    pub const KVM_GET_REGS: u64 = 0x8090_ae81;
}

fn create_mshv_ioctl_rules() -> Result<Vec<SeccompRule>> {
    use mshv::*;
    let common_rules = create_common_ioctl_rules()?;
    let mut arch_rules = or![
        and![Cond::new(1, ArgLen::Dword, Eq, MSHV_UNMAP_GUEST_MEMORY)?],
        and![Cond::new(1, ArgLen::Dword, Eq, MSHV_GET_VP_REGISTERS)?],
        and![Cond::new(1, ArgLen::Dword, Eq, MSHV_SET_VP_REGISTERS)?],
        and![Cond::new(1, ArgLen::Dword, Eq, MSHV_RUN_VP)?],
        and![Cond::new(1, ArgLen::Dword, Eq, MSHV_GET_VP_STATE)?],
        and![Cond::new(1, ArgLen::Dword, Eq, MSHV_ROOT_HVCALL)?],
    ];

    arch_rules.extend(common_rules);

    Ok(arch_rules)
}

fn create_kvm_ioctl_rules() -> Result<Vec<SeccompRule>> {
    use kvm::*;
    let common_rules = create_common_ioctl_rules()?;
    let mut arch_rules = or![
        and![Cond::new(1, ArgLen::Dword, Eq, KVM_SET_REGS)?],
        and![Cond::new(1, ArgLen::Dword, Eq, KVM_SET_FPU)?],
        and![Cond::new(1, ArgLen::Dword, Eq, KVM_RUN)?],
        and![Cond::new(1, ArgLen::Dword, Eq, KVM_GET_REGS)?],
    ];
    arch_rules.extend(common_rules);

    Ok(arch_rules)
}

fn create_ioctl_seccomp_rule() -> Result<Vec<SeccompRule>> {
    match *AVAILABLE_HYPERVISOR {
        HypervisorType::Kvm => create_kvm_ioctl_rules(),
        HypervisorType::HyperVLinux => create_mshv_ioctl_rules(),
        _ => Err(NoHypervisorFound()),
    }
}

fn create_common_ioctl_rules() -> Result<Vec<SeccompRule>> {
    Ok(or![and![Cond::new(1, ArgLen::Dword, Eq, TCGETS)?],])
}
fn syscalls_allowlist() -> Result<Vec<(i64, Vec<SeccompRule>)>> {
    Ok(vec![
        (libc::SYS_mmap, vec![]),
        (libc::SYS_write, vec![]),
        (libc::SYS_close, vec![]),
        (libc::SYS_futex, vec![]),
        (libc::SYS_rt_sigaction, vec![]),
        (libc::SYS_madvise, vec![]),
        (libc::SYS_ioctl, create_ioctl_seccomp_rule()?),
        (libc::SYS_munmap, vec![]),
        (libc::SYS_mprotect, vec![]),
        (libc::SYS_rt_sigprocmask, vec![]),
        (libc::SYS_sched_yield, vec![]),
        (libc::SYS_sigaltstack, vec![]),
        (libc::SYS_getrandom, vec![]),
        (libc::SYS_exit, vec![]),
        (libc::SYS_rt_sigreturn, vec![]),
        (libc::SYS_clock_nanosleep, vec![]),
    ])
}

/// Creates a `BpfProgram` for a `SeccompFilter` over specific syscalls/`SeccompRule`s
/// intended to be applied in the Hypervisor Handler thread - i.e., over untrusted guest code
/// execution.
///
/// Note: This does not provide coverage over the Hyperlight host, which is why we don't need
/// `SeccompRules` for operations we definitely perform but are outside the handler thread
/// (e.g., `KVM_SET_USER_MEMORY_REGION`, `KVM_GET_API_VERSION`, `KVM_CREATE_VM`,
/// or `KVM_CREATE_VCPU`).
pub(crate) fn get_seccomp_filter_for_hypervisor_handler() -> Result<BpfProgram> {
    Ok(SeccompFilter::new(
        syscalls_allowlist()?.into_iter().collect(),
        SeccompAction::KillThread, // non-match syscall will kill the hypervisor handler thread
        SeccompAction::Allow,      // match syscall will be allowed
        std::env::consts::ARCH.try_into().unwrap(),
    )
    .and_then(|filter| filter.try_into())?)
}
