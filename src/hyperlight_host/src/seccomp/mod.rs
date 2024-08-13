// For more information on seccomp and its implementation in Hyperlight,
// refer to: https://github.com/deislabs/hyperlight/blob/dev/docs/seccomp.md

/// This module defines all seccomp filters (i.e., used for blockage of non-specified syscalls)
/// needed for execution of guest code within Hyperlight through a syscalls allow-list.
pub(crate) mod guest;

// The credit on the creation of the macros below goes to the cloud-hypervisor team
// (https://github.com/cloud-hypervisor/cloud-hypervisor/blob/main/vmm/src/seccomp_filters.rs)

/// Shorthand for chaining `SeccompCondition`s with the `and` operator  in a `SeccompRule`.
/// The rule will take the `Allow` action if _all_ the conditions are true.
#[macro_export]
macro_rules! and {
    ($($x:expr),*) => (SeccompRule::new(vec![$($x),*]).unwrap())
}

/// Shorthand for chaining `SeccompRule`s with the `or` operator in a `SeccompFilter`.
#[macro_export]
macro_rules! or {
    ($($x:expr,)*) => (vec![$($x),*]);
    ($($x:expr),*) => (vec![$($x),*])
}
