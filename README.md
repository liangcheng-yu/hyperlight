<div align="center">
  <h1>Hyperlight</h1>
  <img src="docs/assets/hl-tentative-logo.png" width="150px" />
  <p>
    <strong>Hyperlight is a lightweight Virtual Machine Manager that can be hosted in an application. Its purpose is to enable applications to safely run untrusted or third party code within a VM Partition with very low latency/overhead.
    </strong>
  </p>
</div>

> WARNING: There is no implied Azure support for this project. Hyperlight is nascent with a potentially unstable API. Support is provided on a best-effort basis by its developers.

---

## Overview

Hyperlight is an SDK for creating _micro virtual machines_ (VMs) or _sandboxes_ intended for executing arbitrary code by leveraging the [Windows Hypervisor Platform](https://docs.microsoft.com/en-us/virtualization/api/#windows-hypervisor-platform) on Windows and Hyper-V (MSHV) or [KVM](https://linux-kvm.org/page/Main_Page) on Linux without a kernel or operating system. The functionality of a guest is limited to the APIs provided by the host (e.g., `printf`, `malloc`, etc.). The host can also provide additional APIs to the guest, which are called _host functions_.

Below is an example of how Hyperlight host Rust library can be used to run a simple guest application:

```rust
use std::{thread, sync::{Arc, Mutex}};

use hyperlight_common::flatbuffer_wrappers::function_types::{ParameterValue, ReturnType};
use hyperlight_host::{UninitializedSandbox, MultiUseSandbox, func::HostFunction0, sandbox_state::transition::Noop, sandbox_state::sandbox::EvolvableSandbox};

fn main() -> hyperlight_host::Result<()> {
    // Create an uninitialized sandbox with a guest binary
    let mut uninitialized_sandbox = UninitializedSandbox::new(
        hyperlight_host::GuestBinary::FilePath(hyperlight_testing::simple_guest_as_string().unwrap()),
        None, // default configuration
        None, // default run options
        None, // default host print function
    )?;

    // Register a host functions
    fn sleep_5_secs() -> hyperlight_host::Result<()> {
        thread::sleep(std::time::Duration::from_secs(5));
        Ok(())
    }

    let host_function = Arc::new(Mutex::new(sleep_5_secs));

    host_function.register(&mut uninitialized_sandbox, "Sleep5Secs")?;
    // Note: This function is unused, it's just here for demonstration purposes

    // Initialize sandbox to be able to call host functions
    let mut multi_use_sandbox: MultiUseSandbox = uninitialized_sandbox.evolve(Noop::default())?;

    // Call guest function
    let message = "Hello, World! I am executing inside of a VM :)\n".to_string();
    let result = multi_use_sandbox.call_guest_function_by_name(
        "PrintOutput", // function must be defined in the guest binary
        ReturnType::Int,
        Some(vec![ParameterValue::String(message.clone())]),
    );

    assert!(result.is_ok());

    Ok(())
}
```

For additional examples of using the Hyperlight host Rust library, see the [./src/hyperlight_host/examples](./src/hyperlight_host/examples) directory.

For examples of guest applications, see the [./src/tests/Guests](./src/tests/Guests) directory for C guests and the [./src/tests/rust_guests](./src/tests/rust_guests) directory for Rust guests.

> Note: Hyperlight guests can be written using the Hyperlight Rust or C Guest libraries.

## Hyperlight-Wasm

[Hyperlight Wasm](https://github.com/deislabs/hyperlight-wasm) is a sibling project of Hyperlight designed to make it easy for users to run arbitrary Wasm modules within a Hyperlight sandbox.


## Repository Structure

- Hyperlight Host Libraries (i.e., the ones that create and manage the VMs)
  - [src/hyperlight_host](./src/hyperlight_host) - This is the Rust Hyperlight host library.


- Hyperlight Guest Libraries (i.e., the ones to make it easier to create guests that run inside the VMs)
  - [src/HyperlightGuest](./src/HyperlightGuest) - This is the C Hyperlight guest library.
  - [src/hyperlight_guest](./src/hyperlight_guest) - This is the Rust Hyperlight guest library.


- Test Guest Applications:
    - [src/test/Guests](./src/tests/Guests) - This directory contains two Hyperlight Guest programs written in C, which are intended to be launched within partitions as "guests".
    - [src/test/rust_guests](./src/tests/rust_guests) - This directory contains two Hyperlight Guest programs written in Rust, which are intended to be launched within partitions as "guests".


- Tests:
    - [src/hyperlight-testing](./src/hyperlight_testing/) - Shared testing code for Hyperlight projects build int Rust.


- Miscellaneous:
  - [src/HyperlightDependencies](./src/HyperlightDependencies) - This directory contains a .NET assembly which can be used to build a wrapper around Hyperlight such as [Hyperlight WASM](https://github.com/deislabs/hyperlight-wasm).

## Try it yourself!

You can run Hyperlight on:
    - [Linux with KVM][kvm].
    - [Linux with MSHV][azure_linux].
    - [Windows with Windows Hypervisor Platform (WHP) or Hyper-V (MSHV)][whp]. If you don't have WHP, you can use our "in-process" mode, which is intended for development purposes only.
    - Windows Subsystem for Linux 2 ([WSL2][wsl2]) with [KVM][wsl2-kvm].

After having an environment with a hypervisor setup, running the example has the following pre-requisites:

1. On Linux, you'll most likely need build essential. For Mariner, run `sudo dnf install build-essential`. For Ubuntu, run `sudo apt install build-essential`
2. [Rust](https://www.rust-lang.org/tools/install). Install toolchain v1.78.0 or later. Also, install the `x86_64-pc-windows-msvc` and `x86_64-unknown-none` targets with `rustup target add <target>` for each; these are needed to build the test guest binaries. (Note: install both targets on either Linux or Windows: Hyperlight can load ELF or PE files on either OS, and the tests/examples are built for both).
3. [just](https://github.com/casey/just). `cargo install just` .
4. [clang and LLVM](https://clang.llvm.org/get_started.html).
   - On Mariner, run `sudo install clang16 clang16-tools-extra lld16`.
   - On Ubuntu, run:
       ```sh
       wget https://apt.llvm.org/llvm.sh
       chmod +x ./llvm.sh
       ./llvm.sh 17 all
       ln -s /usr/lib/llvm-17/bin/clang-cl /usr/bin/clang-cl
       ln -s /usr/lib/llvm-17/bin/llvm-lib /usr/bin/llvm-lib
       ln -s /usr/lib/llvm-17/bin/lld-link /usr/bin/lld-link
       ln -s /usr/lib/llvm-17/bin/llvm-ml /usr/bin/llvm-ml
       ```
     - On Windows, see [this](https://learn.microsoft.com/en-us/cpp/build/clang-support-msbuild?view=msvc-170).
5. [The Azure CLI](https://learn.microsoft.com/en-us/cli/azure/install-azure-cli).

Now, while Hyperlight is closed source, you'll need to connect to our [Hyperlight Cargo feeds](https://dev.azure.com/AzureContainerUpstream/hyperlight/_artifacts/feed/hyperlight_packages_test) to build the Rust Hyperlight library. To do so, run the following commands:

```sh
git clone https://github.com/deislabs/hyperlight.git # or, git clone git@github.com:deislabs/hyperlight.git
az login # necessary to connect to the Hyperlight Cargo feeds while Hyperlight is closed-source
just cargo-login # connect to the Hyperlight Cargo feeds
cargo update --dry-run # verify access to the Hyperlight Cargo feeds
```

> Note: To gain access to the cargo feeds, navigate to [IDWeb](http://aka.ms/idweb) and join the **Hyperlight-Cargo-Readers** security group.

Then, we are ready to build and run the example:

```sh
just build-rust # build the Rust Hyperlight library
just build-and-move-rust-guests # build the test guest binaries
cargo run --example hello-world # runs the example
```

If all worked as expected, you should the following message in your console:

```text
Hello, World! I am executing inside of a VM :)
```

> Note: For general Hyperlight development, you'll most likely also need these additional pre-requisites:
> - [cbindgen](https://github.com/eqrion/cbindgen). `cargo install cbindgen`
> - For Windows, install the Visual Studio 2022 build tools. You can find them [here](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022).
> - flatcc (Flatbuffer C compiler): for instructions, see [here](https://github.com/dvidelabs/flatcc).
> - flatc (Flatbuffer compiler for other languages): for instructions, see [here](https://github.com/google/flatbuffers).

## Running Hyperlight's entire test-suite

If you are interested in contributing to Hyperlight, running the entire test-suite is a good way to get started. To do so, on your console, run the following commands:

```sh
git clone https://github.com/deislabs/hyperlight.git # or, git clone git@github.com:deislabs/hyperlight.git
just rg
just cg
just build
just test # runs the tests
```

## Troubleshooting

### `NoHypervisorFound`

(1) For running tests, we require an environment variable to be set, which you could be missing (e.g., `KVM_SHOULD_BE_PRESENT=true`).

(2) If you have a Hypervisor device (e.g., `/dev/kvm` or `/dev/mshv`) setup, it could just be a permissions issue.
Update your permissions (e.g., `sudo chmod 666 /dev/kvm` or set up a KVM group as shown [here](https://help.ubuntu.com/community/KVM/Installation)).

(3) You really don't have a Hypervisor. If this is the case, look for instructions for your specific platform to get
setup (here's an example for [Ubuntu KVM](https://ubuntu.com/blog/kvm-hyphervisor)).

## More Information

For more information, please refer to our compilation of documents in the [`docs/` directory](./docs/README.md).

## Code of Conduct

See the [Code of Conduct](./CODE_OF_CONDUCT.md).

[wsl2]: https://docs.microsoft.com/en-us/windows/wsl/install
[wsl2-kvm]: https://boxofcables.dev/kvm-optimized-custom-kernel-wsl2-2022/
[kvm]: https://help.ubuntu.com/community/KVM/Installation
[azure_linux]: ./docs/mariner-mshv-setup
[whp]: https://devblogs.microsoft.com/visualstudio/hyper-v-android-emulator-support/#1-enable-hyper-v-and-the-windows-hypervisor-platform
