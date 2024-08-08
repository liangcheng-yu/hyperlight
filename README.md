# Hyperlight - An lightweight Hypervisor Sandbox

_Hyperlight_ is a lightweight Hypervisor Sandbox. Its purpose is to enable applications to safely run untrusted or third party code within a HyperVisor Partition with very low latency/overhead.

This initial release is designed to be used as an SDK in a dotnet application, future versions will be made available for other languages and frameworks, we are currently porting the majority of Hyperlight to Rust with the aim of creating a Rust implementation with a C API that interops with language-specific SDKs.

Hyperlight supports running applications using the [Windows Hypervisor Platform](https://docs.microsoft.com/en-us/virtualization/api/#windows-hypervisor-platform) on Windows and Hyper-V on Linux (mshv). Currently, the only way to run mshv is on the Mariner distribution, see our [mshv setup instructions][mariner] for more information.

>WARNING: This is experimental code. It is not considered production-grade by its developers, neither is it "supported" software.

>If you are a Hyperlight developer, and are looking to release a new Cargo version, please see [docs/release.md](./docs/release.md).

## The Hyperlight Sandbox and Guest Applications

Hyperlight runs applications in a "sandbox" that it provides. A sandbox is a VM partition along with a very small runtime Hyperlight provides. Since Hyperlight runs guest applications within a Sandbox without a kernel or operating system, all guest applications must be written specifically for the Hyperlight runtime. The intention is that most developers will not need to write such applications, but will take advantage of pre-existing applications written for Hyperlight.

## Hyperlight-Wasm

One primary scenario is to run WebAssembly (WASM) modules. A sibling project called [Hyperlight WASM](https://github.com/deislabs/hyperlight-wasm) is available to make it easy for users to build tools that run arbitrary WASM modules within a Hyperlight sandbox. Hyperlight WASM is an example of a "guest" application, but a very general one that comes with a full WASM runtime and provides a simple API to run WASM modules within a sandbox.

## Projects Inside This Repository

This repo contains Hyperlight along with a couple of sample guest applications that can be used to either test or try it out:

- [src/Hyperlight](./src/Hyperlight) - This is the "host", which launches binaries within a Hypervisor partition.
- [src/HyperlightGuest](./src/HyperlightGuest) - This is a library to make it easy to write Hyperlight Guest programs.
- [src/NativeHost](./src/examples/NativeHost) - This is a "driver" program used for testing. It knows how to run the Hyperlight Guest programs applications that live within the `src/test/Guests` directory (see below) within sandboxes. If you are developing Hyperlight itself, you'll need this program, but if you're using the library to build your own applications, you won't need this project.
- [src/HyperlightSurrogate](./src/HyperlightSurrogate) - See [below](#hyperlightsurrogate) for more information.
- [src/tests](./src/tests) - Tests for the host
  - [src/test/Guests](./src/tests/Guests) This directory contains two Hyperlight Guest programs written in C, which are intended to be launched within partitions as "guests".
  - Some of the Rust tests use [proptest](https://docs.rs/proptest/latest/proptest/index.html) to do property-based testing (a [QuickCheck](https://en.wikipedia.org/wiki/QuickCheck) variant specifically). Read more about `proptest` in the [`proptest` book](https://altsysrq.github.io/proptest-book/), and in this useful [LogRocket blog post](https://blog.logrocket.com/property-based-testing-in-rust-with-proptest/).
- [src/HyperlightDependencies](./src/HyperlightDependencies) - This directory contains a dotnet assmebly which can be used to build a wrapper around Hyperlight such as  [Hyperlight WASM](https://github.com/deislabs/hyperlight-wasm).
- [src/hyperlight-capi](./src/hyperlight_capi/) - C-API bindings for the in-progress rewrite of the Hyperlight host into Rust.
- [src/hyperlight-host](./src/hyperlight_host) - This is the in-progress rewrite of the Hyperlight host into Rust. See [the design document](https://hackmd.io/@arschles/hl-rust-port) for more information about this work, and see below for details on how to use this code.
- [src/hyperlight-testing](./src/hyperlight_testing/) - Shared testing code for Hyperlight projects build int Rust.

### HyperlightSurrogate

hyperlight_surrogate.exe is a tiny Rust application we use to create multiple virtual machine (VM) partitions per process when running on Windows with the Windows Hypervisor Platform (WHP, e-g Hyper-V). This binary has no functionality. Its purpose is to provide a running process into which memory will be mapped via the `WHvMapGpaRange2` Windows API. Hyperlight does this memory mapping to pass parameters into, and fetch return values out of, a given VM partition.

> Note: The use of surrogates is a temporary workaround on Windows until WHP allows us to create more than one partition per running process.

These surrogate processes are managed by the host via the [surrogate_process_manager](./src/hyperlight_host/src/hypervisor/surrogate_process_manager.rs) which will launch several of these surrogates (up to the 512), assign memory to them, then launch partitions from there, and reuse them as necessary.

hyperlight_surrogate.exe gets built during `hyperlight_host`'s build script, gets embedded into the `hyperlight_host` rust library via [rust-embed](https://crates.io/crates/rust-embed), and is extracted at runtime next to the executable when the surrogate process manager is initialized.

Initially, HyperlightSurrogate.exe was written in C and performed the same function as the Rust version. The C version is still referenced in the .NET solution (`.sln` file).
To build HyperlightSurrogate.exe run the following from a [Visual Studio Command Prompt](https://docs.microsoft.com/en-us/visualstudio/ide/reference/command-prompt-powershell?view=vs-2022)

```cmd
msbuild hyperlight.sln -target:HyperlightSurrogate:Result /p:Configuraiton={Debug|Release}
```

## Quickstart

Here is the quickest way to try out Hyperlight:

1. Get the latest release for [Windows](https://github.com/deislabs/hyperlight/releases/download/latest/windows-x64.zip) or [Linux](https://github.com/deislabs/hyperlight/releases/download/latest/linux-x64.tar.gz).
   1. If you have GitHub's `gh` CLI, run this command: `gh release download latest` from within a new folder, as it will download several files at once.
2. Extract the archive to a location on your computer
3. Run the NativeHost.exe or NativeHost in the extracted directory.

Note: You can also run the linux version using WSL2 on Windows. At present there is no version available for macOS.

To use KVM on Linux, ensure you have it installed. If you don't, follow instructions [here](https://help.ubuntu.com/community/KVM/Installation).

To use mshv on Linux follow the instructions [here][mariner].

The code for the NativeHost application is available [here](https://github.com/deislabs/hyperlight/blob/main/src/examples/NativeHost/Program.cs).

On Windows if you don't have Windows Hypervisor Platform enabled then the example application will only run in 'in process' mode, this mode is provided for development purposes and is not intended to be used in production. If you want to see the example running code in a Hypervisor partition then you will need to either install [Windows Hypervisor Platform](https://devblogs.microsoft.com/visualstudio/hyper-v-android-emulator-support/#1-enable-hyper-v-and-the-windows-hypervisor-platform). NOTE - To enable WHP on Windows Server you need to enable the Windows Hypervisor Platform feature using PowerShell `Enable-WindowsOptionalFeature -Online -FeatureName HyperVisorPlatform`.

On Linux (including WSL2) you must install [KVM](https://help.ubuntu.com/community/KVM/Installation) (see [here](https://boxofcables.dev/kvm-optimized-custom-kernel-wsl2-2022/) for instrucitons how to build an accerlerted custom kernel for WSL2). If you have access to CBL-Mariner with HyperV this will also work.

## Development

### Windows

Currently the complete solution including tests and examples will only build on Windows with Visual Studio 2022 or the Visual Studio 2022 Build Tools along with dotnet 6.0, this is because the `HyperlightGuest` project must be compiled with Microsoft Visual C compiler at present, in additon the test and example projects are dependent upon the test Hyperlight Guest applications that also require MSVC. In addition you will need the [prerequisites](#prerequisites) installed.

#### Windows Prerequisites

1. [Rust](https://www.rust-lang.org/tools/install)
1. [just](https://github.com/casey/just).  `cargo install just`. Do not install `just` with Chocolately because it installs an older incompatible version.
1. [cbindgen](https://github.com/eqrion/cbindgen) `cargo install cbindgen`
1. [Clang](https://clang.llvm.org/get_started.html).  If you have Visual Studio instructions are [here](https://docs.microsoft.com/en-us/cpp/build/clang-support-msbuild?view=msvc-170).
1. [pwsh](https://github.com/PowerShell/PowerShell)
1. [dotnet](https://learn.microsoft.com/en-us/dotnet/core/install/windows)
1. [Set up the Hyperlight Cargo Feed](#hyperlight-cargo-feed)
1. [Set Up simpleguest.exe and callbackguest.exe](#simpleguestexe-dummyguestexe-callbackguestexe)

 Create powershell function to use developer shell as shell:

 1. Edit `$PROFILE`
 1. Add the following to the profile, this assumes that you have installed clang via Visual Studio and are happy to add the developer shell to your default pwsh profile.

 Note: You may not have the `$PROFILE` file created yet and you may have to create a new file and then update it.

Gather the vs instance id for your dev environment by running `vswhere.exe -legacy -prerelease -format json` and look for the instance id of your VS installation. (vswhere.exe can be downloaded from [here](https://github.com/microsoft/vswhere/releases))

Replace the <instance_id> appropriately and copy it to the script file pointed by the $PROFILE.
 ```PowerShell
Import-Module "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\Common7\Tools\Microsoft.VisualStudio.DevShell.dll"
Enter-VsDevShell <instance_id> -SkipAutomaticLocation -DevCmdArguments "-arch=x64 -host_arch=x64"
 ```
#### Visual Studio 2022

If you do not have Visual Studio 2022  ou can find it [here](https://visualstudio.microsoft.com/downloads/).

Clone the repo, launch Visual Studio open the `Hyperlight.sln` file and build.

Run the tests from Test Explorer.

Run additional tests, open a command prompt and run the following commands:

``` console
just init
just build-capi
just test-rust
just test-capi
just test-dotnet-hl
```

#### Visual Studio Build tools

Install the build tools from [here](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022).

Clone the repo.

Open a [Visual Studio Command Prompt](https://docs.microsoft.com/en-us/visualstudio/ide/reference/command-prompt-powershell?view=vs-2022), cd to the hyperlight repo root directory and run `msbuild hyperlight.sln /p:BuildHyperLightHost=true`.

Test by running:

``` console
just init
just build-capi
just test-rust
just test-capi
just test-dotnet
```

#### Visual Studio Code

You can also use Visual Studio code, to do this make sure that you start Visual Studio Code from a [Visual Studio Command Prompt](https://docs.microsoft.com/en-us/visualstudio/ide/reference/command-prompt-powershell?view=vs-2022) and then open the folder that you cloned the repo to.

#### Without Visual Studio or the Visual Studio Build Tools

```console
git clone git@github.com:deislabs/hyperlight.git
cd hyperlight
# Hyperlight uses submodules to pull in some dependencies such as munit
# If you see munit errors when running tests, make sure you have the submodules cloned
git submodule update --init
just init
just build
just test-rust
just test-capi
```

#### Running dotnet tests

```console
cd src/tests/Hyperlight.Tests
dotnet test
```

#### Running dotnet example

```console
cd src/examples/NativeHost
dotnet run
```

### Linux or WSL2

#### Linux Prerequisites

1. Build-Essential. `sudo apt install build-essential` or `sudo dnf install build-essential` on [Mariner][mariner]
1. [Rust](https://www.rust-lang.org/tools/install)
1. [just](https://github.com/casey/just).  `cargo install just` .
1. [cbindgen](https://github.com/eqrion/cbindgen) `cargo install cbindgen`
1. [Clang](https://clang.llvm.org/get_started.html). `sudo apt install clang` or `sudo dnf install clang` on [Mariner][mariner].
1. [dotnet](https://learn.microsoft.com/en-us/dotnet/core/install/linux). `sudo apt install dotnet-sdk-6.0` or `sudo dnf install dotnet-sdk-6.0` on [Mariner][mariner].

If you receive a 'GPG check FAILED' error when trying to install dotnet-sdk-6.0 (especially on Mariner), follow [these steps](https://github.com/dotnet/docs/blob/main/docs/core/install/linux-scripted-manual.md#manual-install) to manually install it. You may also need to run `sudo dnf install libicu`

**Running the build**

```console
git clone git@github.com:deislabs/hyperlight.git
cd hyperlight
```

1. [Set up the Hyperlight Cargo Feed](#hyperlight-cargo-feed)
1. [Set Up simpleguest.exe and callbackguest.exe](<#simpleguest.exe-dummyguest.exe-callbackguest.exe>)

```
# Hyperlight uses submodules to pull in some dependencies such as munit
# If you see munit errors when running tests, make sure you have the submodules
# cloned by running the below command
just init
# then you can build using
just build
```

**Running tests**

```
just test-rust
just test-capi
```

## Hyperlight Cargo Feed

The hyperlight Rust projects currently require connecting to Microsoft internal cargo feeds to pull some dependencies.
To do do this please ensure the following:

1. You have access to the [AzureContainerUpstream Hyperlight_packages Cargo feed]('https://dev.azure.com/AzureContainerUpstream/hyperlight/_artifacts/feed/hyperlight_packages_test')

    - To gain access please join the navigate to [IDWeb](http://aka.ms/idweb) and join the **Hyperlight-Cargo-Readers** security group.

1. You have the 'az cli' installed and are logged in to AzureDevops

    - (https://learn.microsoft.com/en-us/cli/azure/install-azure-cli)

1. You have the rust toolchain v1.78.0 (or later) installed

To connect to the cargo feeds run the following commands from the root of the repo:

```console
az login
just cargo-login
```

To verify access to our cargo feeds run:

```console
cargo update --dry-run
```

See [publishing-to-cargo.md](./docs/publishing-to-cargo.md) for more information.

## simpleguest.exe, dummyguest.exe, callbackguest.exe

To run the dotnet tests and examples you will need the dummyguest.exe, simpleguest.exe, and callbackguest.exe applications. Run
```bash
just build-and-move-rust-guests
```
to build dummyguest.exe, simpleguest.exe, and callbackguest.exe.

Then run this script:

```bash
./dev/test-guests.sh
```

## Debugging The Hyperlight Guest Applications or GuestLibrary

To debug the guest applications or library the Sandbox instance needs to be created with the option flag `SandboxRunOptions.RunFromGuestBinary`.

### Debugging in Visual Studio

Mixed mode debugging in Visual Studio is enabled in the solution, this means that you can set breakpoints in managed and/or native code, step into native code from managed code etc. during debugging.

### Debugging in Visual Studio Code

Visual Studio Code does not currently support mixed mode debugging, to debug guest applications in Visual Studio Code you need to choose the `Debug Native Host` debugging task when starting a debug session.

### Getting debug print output of memory configuration, virtual processor register state and other information

Setting the feature `print_debug` and running a debug build will result in some debug output being printed to the console. Amongst other things this output will show the memory configuration and virtual processor register state.

To enable this permantly in the rust analyzer for Visual Studio Code so that this output shows when running tests using `Run Test` option add the following to your `settings.json` file:

```json
"rust-analyzer.runnables.extraArgs": [
    "--features=print_debug"
],
```

Alternatively this can be enabled when running a test from the command line e.g:

```console
cargo test --package hyperlight_host --test integration_test --features print_debug -- static_stack_allocate --exact --show-output
```

### Dumping the memory configuration, virtual processor register state and memory contents on a crash or unexpected VM Exit

To dump the details of the memory configuration, the virtual processors register state and the contents of the VM memory set the feature `dump_on_crash` and run a debug build. This will result in a dump file being created in the temporary directory. The name and location of the dump file will be printed to the console and logged as an error message.

There are no tools at this time to analyze the dump file, but it can be useful for debugging.

## Code of Conduct

This project has adopted the [Microsoft Open Source Code of
Conduct](https://opensource.microsoft.com/codeofconduct/).

For more information see the [Code of Conduct
FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or contact
[opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

[mariner]: ./docs/mariner-mshv.md
