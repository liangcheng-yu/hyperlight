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

- [src/Hyperlight](./src/HyperLight) - This is the "host", which launches binaries within a Hypervisor partition.
- [src/NativeHost](./src/examples/NativeHost) - This is a "driver" program used for testing. It knows how to run the two "guest" applications that live within the `test/` directory (see below) within sandboxes. If you are developing Hyperlight itself, you'll need this program, but if you're using the library to build your own applications, you won't need this project.
- [src/HyperlightSurrogate](./src/HyperlightSurrogate) - This is a tiny application that is simply used as a sub-process for the host. When the host runs on Windows with the Windows Hypervisor Platform (e.g. Hyper-V), it launches several of these surrogates, assigns memory to them, and then launches partitions from there.
- [src/tests](./src/tests) - Tests for the host
  - This directory contains two applications written in C, which are intended to be launched within partitions as "guests".

## Quickstart

Here is the quickest way to try out Hyperlight:

1. Get the latest release for [Windows](https://github.com/deislabs/hyperlight/releases/download/latest/windows-x64.zip) or [Linux](https://github.com/deislabs/hyperlight/releases/download/latest/linux-x64.tar.gz).
   1. If you have GitHub's `gh` CLI, run this command: `gh release download latest` from within a new folder, as it will download several files at once.
2. Extract the archive to a location on your computer
3. Run the NativeHost.exe or NativeHost in the extracted directory.

Note: You can also run the linux version using WSL2 on Windows. At present their is no version available for macOS.

The code for the NativeHost application is available [here](https://github.com/deislabs/hyperlight/blob/main/src/examples/NativeHost/Program.cs).

If you dont have Windows Hypervisor Platform enabled or KVM installed then the example application will only run in 'in process' mode, this mode is provided for development purposes and is not intended to be used in production. If you want to see the example running code in a Hypervisor partition then you will need to either install [Windows Hypervisor Platform](https://devblogs.microsoft.com/visualstudio/hyper-v-android-emulator-support/#1-enable-hyper-v-and-the-windows-hypervisor-platform) or [KVM](https://help.ubuntu.com/community/KVM/Installation). NOTE - To enable WHP on Windows Server you need to enable the Windows Hypervisor Platform feature using PowerShell `Enable-WindowsOptionalFeature -Online -FeatureName HyperVisorPlatform`.

## Building and testing Hyperlight

Currently the complete solution including tests and examples will only build on Windows with Visual Studio 2019 (or later) or the Visual Studio 2022 Build Tools along with dotnet 5.0 and dotnet 6.0, this is becasue the test and example projects are dependent upon a couple of projects that currently need to be compiled with the Microsoft Visual C compiler. 

If you do not have these tools and wish to install them you can find Visual Studio 2019 (https://visualstudio.microsoft.com/downloads/) and the build tools [here](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022).

To use Visual Studio clone the repo and run the following command and open the Hyperlight.sln file. 

You can also use Visual Studio code, to do this make sure that you start Visual Studio Code from a [Visual Studio Command Prompt](https://docs.microsoft.com/en-us/visualstudio/ide/reference/command-prompt-powershell?view=vs-2022) and then open the folder that you cloned the repo to.

If you want to build/test Hyperlight without installing Visual Studio or the Visual Studio buld tools or on Linux then you can do this by following the instructions below.

### Building and Hyperlight using only dotnet on Linux or Windows

Hyperlight will build using the `dotnet build` command on any machine that has the [dotnet 5.0 SDK](https://dotnet.microsoft.com/en-us/download/dotnet/5.0) and/or [dotnet 6.0 SDK](https://dotnet.microsoft.com/en-us/download/dotnet/5.0) installed:

```console
git clone git@github.com:deislabs/hyperlight.git
cd src/Hyperlight
```

To build for dotnet 5.0:
```
dotnet build -f net5.0
```

To build for dotnet 6.0:
```
dotnet build -f net6.0
```

To build for both:
```
dotnet build
```

To run the tests and examples you will need to download the simpleguest.exe and callbackguest.exe applications from [here] (https://github.com/deislabs/hyperlight/releases).

### Running tests

```console
cd src/tests/Hyperlight.Test
dotnet test
```

### Running examples

```console
cd src/examples/NativeHost
dotnet run
```

## Debugging Guest Applications

To debug guest applications the Sandbox needs to be created with the option flag `SandboxRunOptions.RunFromGuestBinary`.

### Debugging in Visual Studio

Mixed mode debugging in Visual Studio is enabled in the solution, this means that you can set breakpoints in managed and/or native code, step into native code from managed code etc. during debugging. 

### Debugging in Visual Studio Code

Visual Studio Code does not currently support mixed mode debugging, to debug guest applications in Visual Studio Code you need to choose the `Debug Native Host` debuggin task when starting a debug session.

## Code of Conduct

This project has adopted the [Microsoft Open Source Code of
Conduct](https://opensource.microsoft.com/codeofconduct/).

For more information see the [Code of Conduct
FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or contact
[opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.
