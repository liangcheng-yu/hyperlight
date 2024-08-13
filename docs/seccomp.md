# Seccomp in Hyperlight

Currently (2024-07-18), the execution of guest code in Hyperlight is secured via seccomp (i.e., secure computing). Seccomp works via the usage of a BPF (Berkeley Packet Filter) Program that applies rules on the execution of syscalls. For example, for Hyperlight, we've tested what syscalls are needed for the execution of guest code and, by pairing that with seccomp rules (e.g., specific `ioctl` syscall parameters, like `KVM_RUN`), we can choose what to allow and what to trap.

It's important to note that we are applying no filters over the execution of the Hyperlight host - as of writing this, only the Hyperlight guest execution (i.e., whatever happens inside the Hypervisor handler thread) is secured via seccomp. Although, that said, the filters do apply to host functions called from a guest. At some point, [we want to add seccomp filters over the host entirely](https://github.com/deislabs/hyperlight/issues/1471).

It is likely that, as Hyperlight grows, there will be more functionality added onto it, and we will need to be more permissive with the syscalls we allow. The process of figuring out what syscalls got trapped isn't necessarily trivial, so this document aims to aid in the process of figuring that out to build upon our current BPF Program.

## I added a feature onto Hyperlight, and got a syscall trap. What now?

When you get a syscall trapped, you will see an error like so:
```text
called `Result::unwrap()` on an `Err` value: DisallowedSyscall
```

With this happening, there are two possible options:
(1) We are missing an actual syscall, or
(2) We are missing a permission of a specific parameter of the `ioctl` syscall.

To figure out which one is your case, change our [`SeccompFilter`](https://github.com/deislabs/hyperlight/blob/dev/src/hyperlight_host/src/seccomp/guest.rs#L103) to `SeccompAction::Log` non-matched syscalls instead of `SeccompAction::KillThread`.

Once you do that and re-run, your code should pass. But our job is not done!

Let's check what got trapped, so we can go back to trapping possibly malicious syscalls later.

You can check what was trapped with:
```shell
journalctl | grep SECCOMP
```
> Note: `journalctl | grep SECCOMP` looks very different on MSHV and KVM. This tutorial covers how it shows on KVM, so more experimentation might be needed on MSHV.
This will output something like:
```text
<date> <your-user> audit[62172]: SECCOMP auid=4294967295 uid=1000 gid=1000 ses=4294967295 subj=kernel pid=62172 comm="<function-hint>" exe="<some executable>" sig=0 arch=c000003e syscall=1 compat=0 ip=0x7cbdbe5b132f code=0x7ffc0000
```

You can see we logged a call to syscall number 1. You can then refer to a syscall table of your target architecture to check what syscall that number corresponds to (here's an [example](https://github.com/rust-lang/libc/blob/0e28c864c25d2e9b0ab082947445efccef213da4/src/unix/linux_like/linux/gnu/b64/x86_64/not_x32.rs#L70)). From the example syscall table, we see that 1 corresponds to the syscall `write`. With that being the case, we fall onto option (1). And, all we need to do to support that syscall is add to our [syscall allowlist](https://github.com/deislabs/hyperlight/blob/dev/src/hyperlight_host/src/seccomp/guest.rs#L101), like: `(libc::SYS_write, vec![])`. Note, the empty vector here means that there are no extra `SeccompRules` on top of this syscall allow. The only syscall we add rules over is the `ioctl` one because we only want to permit `ioctl` calls with specific parameters (e.g., `KVM_RUN`).

Continuing, if you found any syscall number other than the one correspondent to `ioctl` (usually, 16), the process is the same. However, if you found you trapped over the `ioctl` syscall, it means you need to add a permission to another `ioctl` parameter(s) like in [here](https://github.com/deislabs/hyperlight/blob/dev/src/hyperlight_host/src/seccomp/guest.rs#L46). To figure out what parameter you need to add, I'd recommend running the test you trapped over with a `strace` around it filtering for `ioctl`s. Here's an example:

```shell
strace -e trace=ioctl -f -o strace_log.txt cargo test --color=always --test integration_test print_four_args_c_guest --manifest-path /mnt/c/Users/danil/source/repos/hyperlight/src/hyperlight_host/Cargo.toml -- --show-output
```

Then, you use this helpful `awk` routine to see what parameters we used for our `ioctl`s:

```shell
awk '
/ioctl/ {
    if (match($0, /ioctl\([0-9]+, ([^,]+),/, arr)) {
        ioctl_code[arr[1]]++;
    }
}
END {
    for (code in ioctl_code) {
        printf("%s %d\n", code, ioctl_code[code]);
    }
}
' strace_log.txt | sort -nr -k2
```

However, note, this is likely to give you a larger list than you need to support, like:
```text
TCGETS 14
KVM_SET_USER_MEMORY_REGION 8
TIOCGWINSZ 5
KVM_SET_REGS 5
KVM_RUN 3
KVM_GET_REGS 3
KVM_GET_API_VERSION 2
KVM_CHECK_EXTENSION 2
KVM_SET_SREGS 1
KVM_GET_VCPU_MMAP_SIZE 1
KVM_GET_SREGS 1
KVM_CREATE_VM 1
KVM_CREATE_VCPU 1
```

As of writing this, we only allow `ioctl`s on KVM for guest execution with parameters: `KVM_SET_REGS`, `KVM_SET_FPU`, `KVM_RUN`, and `KVM_GET_REGS`. So, clearly, this list doesn't fully represent the smallest subset of `ioctl`s we need for guest execution. That's because we are capturing the `ioctl`s over the entirety of the `cargo test` call. This means we could be capturing `ioctl`s from `cargo`, or, more commonly, from the host execution. My recommendation for figuring out what `ioctl` parameters you need to add is to include them all (for this example, all the ones starting with `KVM`) with a `SeccompAction::Trap` and try removing them one by one to figure out the ones you needed for the test to pass. To obtain the specific values of the `ioctl` parameters like: `pub const KVM_SET_REGS: u64 = 0x4090_ae82`, I'd recommend referring to [cloud-hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor/blob/7b3ffd89a5790190271726b0aba48d4971017218/vmm/src/seccomp_filters.rs#L137).

For MSHV, the `ioctl` parameters don't necessarily match the names you'll see in the `guest.rs` file. Instead, with that `awk` command, you'll probably see something like:

```text
_IOC(_IOC_WRITE 21953
_IOC(_IOC_READ 4959
_IOC(_IOC_READ|_IOC_WRITE 2959
TCGETS 891
TIOCGWINSZ 105
TIOCGPGRP 23
FIONBIO 9
```

Keep that in mind and maybe just refer to `cloud-hypervisor` for `ioctl` parameter trial and error. One thing I'd recommend is to reflect upon the feature you are adding and think about what parameter you might want. For example, when we added `set_fpu` usage onto `KVM`, it makes sense we'd want to add onto our `ioctl` parameters a permission for `KVM_SET_FPU`. 

## Extra Resources

Seccomp filters were added in [this](https://github.com/deislabs/hyperlight/pull/1450) PR and, throughout it, danbugs documented the process of figuring out the needed syscalls and ioctl parameters through PR comments. If all in this tutorial fails, you might find it helpful to refer to that PR as it lists some extra commands that were run to figure all this out.