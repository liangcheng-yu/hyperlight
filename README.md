# Hyperlight - An lightweight Hypervisor Sandbox

_Hyperlight_ is a lightweight Hypervisor Sandbox. Its purpose is to enable applications to safely run untrusted or third party code within a HyperVisor Partition with very low latency/overhead.

This initial release is designed to be used in a dotnet application, future versions will be made available for other languages and frameworks.

Hyperlight currently supports running applications using either the [Windows Hypervisor Platform](https://docs.microsoft.com/en-us/virtualization/api/#windows-hypervisor-platform) on Windows or [KVM](https://www.linux-kvm.org/page/Main_Page) on Linux.

>WARNING: This is experimental code. It is not considered production-grade by its developers, neither is it "supported" software.

## The Hyperlight Sandbox and Guest Applications

Hyperlight runs applications in a "sandbox" that it provides. A sandbox is a VM partition along with a very small runtime Hyperlight provides. Since Hyperlight runs guest applications within a Sandbox without a kernel or operating system, all guest applications must be written specifically for the Hyperlight runtime. The intention is that most developers will not need to write such applications, but will take advantage of pre-existing applications written for Hyperlight.

## Hyperlight-Wasm

One primary scenerio is to run WebAssembly (WASM) modules. A sibling project called [Hyperlight WASM](https://github.com/deislabs/hyperlight-wasm) is available to make it easy for users to build tools that run arbitrary WASM modules within a Hyperlight sandbox. Hyperlight WASM is an example of a "guest" application, but a very general one that comes with a full WASM runtime and provides a simple API to run WASM modules within a sandbox.

## Projects Inside This Repository

This repo contains Hyperlight along with a couple of sample guest applications that can be used to either test or try it out:

- [src/Hyperlight](./src/Hyperlight) - This is the "host", which launches binaries within a Hypervisor partition.
- [src/HyperlightGuest](./src/HyperLightGuest) - This is a library to make it easy to write Hyperlight Guest programs.
- [src/NativeHost](./src/examples/NativeHost) - This is a "driver" program used for testing. It knows how to run the Hyperlight Guest programs applications that live within the `src/test/Guests` directory (see below) within sandboxes. If you are developing Hyperlight itself, you'll need this program, but if you're using the library to build your own applications, you won't need this project.
- [src/HyperlightSurrogate](./src/HyperlightSurrogate) - This is a tiny application that is simply used as a sub-process for the host. When the host runs on Windows with the Windows Hypervisor Platform (WHP, e.g. Hyper-V), it launches several of these surrogates, assigns memory to them, and then launches partitions from there.
  - The use of surrogates is a temporary workaround on Windows until WHP allows us to create more than one partition per running process.
- [src/tests](./src/tests) - Tests for the host
  - [`src/test/Guests](./src/tests/Guests) This directory contains two Hyperlight Guest programs written in C, which are intended to be launched within partitions as "guests".
- [src/HyperlightDependencies](./src/HyperlightDependencies) - This directory contains a dotnet assmebly which can be used to build a wrapper around Hyperlight such as  [Hyperlight WASM](https://github.com/deislabs/hyperlight-wasm).
- [src/hyperlight-host](./src/hyperlight_host) - This is the in-progress rewrite of the Hyperlight host into rust. See [the design document](https://hackmd.io/@arschles/hl-rust-port) for more information about this work, and see below for details on how to use this code.

## Quickstart

Here is the quickest way to try out Hyperlight:

1. Get the latest release for [Windows](https://github.com/deislabs/hyperlight/releases/download/latest/windows-x64.zip) or [Linux](https://github.com/deislabs/hyperlight/releases/download/latest/linux-x64.tar.gz).
   1. If you have GitHub's `gh` CLI, run this command: `gh release download latest` from within a new folder, as it will download several files at once.
2. Extract the archive to a location on your computer
3. Run the NativeHost.exe or NativeHost in the extracted directory.

Note: You can also run the linux version using WSL2 on Windows. At present their is no version available for macOS.

The code for the NativeHost application is available [here](https://github.com/deislabs/hyperlight/blob/main/src/examples/NativeHost/Program.cs).

On Windows if you dont have Windows Hypervisor Platform enabled then the example application will only run in 'in process' mode, this mode is provided for development purposes and is not intended to be used in production. If you want to see the example running code in a Hypervisor partition then you will need to either install [Windows Hypervisor Platform](https://devblogs.microsoft.com/visualstudio/hyper-v-android-emulator-support/#1-enable-hyper-v-and-the-windows-hypervisor-platform). NOTE - To enable WHP on Windows Server you need to enable the Windows Hypervisor Platform feature using PowerShell `Enable-WindowsOptionalFeature -Online -FeatureName HyperVisorPlatform`.

On Linux (including WSL2) you must install [KVM](https://help.ubuntu.com/community/KVM/Installation) (see [here](https://boxofcables.dev/kvm-optimized-custom-kernel-wsl2-2022/) for instrucitons how to build an accerlerted custom kernel for WSL2). If you have access to CBL-Mariner with HyperV this will also work.

## Building and testing the Hyperlight Solution on Windows

Currently the complete solution including tests and examples will only build on Windows with Visual Studio 2022 or the Visual Studio 2022 Build Tools along with dotnet 6.0, this is because the `HyperlightGuest` project must be compiled with Microsoft Visual C compiler at present, in additon the test and example projects are dependent upon the test Hyperlight Guest applications that also require MSVC. In addition you will need the [prerequisites](#prerequisites) installed.

### Visual Studio 2022

If you do not have Visual Studio 2022  ou can find it [here](https://visualstudio.microsoft.com/downloads/).

Clone the repo, launch Visual Studio open the `Hyperlight.sln` file and build.

Run the tests from Test Explorer.

Run additional tests, open a command prompt and run the following commands:

``` console
just init
just build-tests-capi
just test-rust
just test-capi
just test-dotnet-hl
```

### Visual Studio Build tools

Install the build tools from [here](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022).

Clone the repo.

Open a [Visual Studio Command Prompt](https://docs.microsoft.com/en-us/visualstudio/ide/reference/command-prompt-powershell?view=vs-2022), cd to the hyperlight repo root directory and run `msbuild hyperlight.sln /p:BuildHyperLightHost=true`.

Test by running:

``` console
just init
just build-tests-capi
just test-rust
just test-capi
just test-dotnet
```

### Visual Studio Code

You can also use Visual Studio code, to do this make sure that you start Visual Studio Code from a [Visual Studio Command Prompt](https://docs.microsoft.com/en-us/visualstudio/ide/reference/command-prompt-powershell?view=vs-2022) and then open the folder that you cloned the repo to.

## Building and Testing Hyperlight using Linux or Windows without Visual Studio or the Visual Studio Build Tools

Hyperlight will build on any Windows or Linux machine that has the [prerequisites](#prerequisites) installed:

```console
git clone git@github.com:deislabs/hyperlight.git
cd hyperlight
just init
just build
just test-rust
just test-capi
```

To run the  dotnet tests and examples you will need to download the simpleguest.exe and callbackguest.exe applications from [here] (https://github.com/deislabs/hyperlight/releases) and copy them to `src/tests/Guests/simpleguest/x64/debug/simpleguest.exe` and  `src/tests/Guests/callbackguest/x64/debug/callbackguest.exe` respectively. The directories do not exist, so you will need to create them first (note that they are case-sensitive).

### Running dotnet tests

```console
cd src/tests/Hyperlight.Tests
dotnet test
```

### Running dotnet example

```console
cd src/examples/NativeHost
dotnet run
```

## Debugging The Hyperlight Guest Applications or GuestLibrary

To debug the guest applications or library the Sandbox instance needs to be created with the option flag `SandboxRunOptions.RunFromGuestBinary`.

### Debugging in Visual Studio

Mixed mode debugging in Visual Studio is enabled in the solution, this means that you can set breakpoints in managed and/or native code, step into native code from managed code etc. during debugging.

### Debugging in Visual Studio Code

Visual Studio Code does not currently support mixed mode debugging, to debug guest applications in Visual Studio Code you need to choose the `Debug Native Host` debugging task when starting a debug session.

## The Rust Host Rewrite (`hyperlight_host`)

## Prerequisites

### Windows

1. [Rust](https://www.rust-lang.org/tools/install)
1. [Clang](https://clang.llvm.org/get_started.html).  If you have Visual Studio instructions are [here](https://docs.microsoft.com/en-us/cpp/build/clang-support-msbuild?view=msvc-170).
1. [just](https://github.com/casey/just).  `cargo install just` or with chocolatey `choco install just`.
1. [cbindgen](https://github.com/eqrion/cbindgen) `cargo install cbindgen`
1. [pwsh](https://github.com/PowerShell/PowerShell)
1. [dotnet](https://learn.microsoft.com/en-us/dotnet/core/install/windows)


 Create powershell function to use developer shell as shell:

 1. Edit $PROFILE
 1. Add the following to the profile, this assumes that you have installed clang via Visual Studio and are happy to add the developer shell to your default pwsh profile.

 ```PowerShell
Import-Module "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\Common7\Tools\Microsoft.VisualStudio.DevShell.dll"
Enter-VsDevShell 001cb2cc -SkipAutomaticLocation -DevCmdArguments "-arch=x64 -host_arch=x64" 
 ```

### WSL2 or Linux

Prerequisites:

1. [Rust](https://www.rust-lang.org/tools/install)
1. [Clang](https://clang.llvm.org/get_started.html). `sudo apt install clang`.
1. [just](https://github.com/casey/just).  `cargo install just` .
1. [cbindgen](https://github.com/eqrion/cbindgen) `cargo install cbindgen`
1. [dotnet](https://learn.microsoft.com/en-us/dotnet/core/install/linux)

## Code of Conduct

This project has adopted the [Microsoft Open Source Code of
Conduct](https://opensource.microsoft.com/codeofconduct/).

For more information see the [Code of Conduct
FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or contact
[opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.
