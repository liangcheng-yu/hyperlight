# Debugging Hyperlight

Debugging in Hyperlight can be a mixed experience from running in different environments (i.e., Linux/Windows) or different IDEs (i.e., Visual Studio/Visual Studio Code). This document aims to provide a guide to debugging Hyperlight in different environments.

## Debugging Hyperlight guest applications or guest library

On Windows, to debug the guest applications or library the Sandbox instance needs to be created with the option flag `SandboxRunOptions.RunFromGuestBinary`.

## Debugging in Visual Studio

Mixed mode debugging in Visual Studio is enabled in the solution, this means that you can set breakpoints in managed and/or native code, step into native code from managed code and so forth during debugging.

## Debugging in Visual Studio Code

Visual Studio Code does not currently support mixed mode debugging, to debug guest applications in Visual Studio Code you need to choose the `Debug Native Host` debugging task when starting a debug session.

> Note: While this section mostly covers debugging within the context of the C# Hyperlight host library, in Rust, you can also use [dbgtools-win](https://crates.io/crates/dbgtools-win) in parallel with inserting a `wait_for_then_break()` line in code to easily attach a debugger.

## Getting debug print output of memory configuration, virtual processor register state, and other information

Enabling the feature `print_debug` and running a debug build will result in some debug output being printed to the console. Amongst other things this output will show the memory configuration and virtual processor register state.

To enable this permanently in the rust analyzer for Visual Studio Code so that this output shows when running tests using `Run Test` option add the following to your `settings.json` file:

```json
"rust-analyzer.runnables.extraArgs": [
    "--features=print_debug"
],
```

Alternatively, this can be enabled when running a test from the command line:

```sh
cargo test --package hyperlight_host --test integration_test --features print_debug -- static_stack_allocate --exact --show-output
```

## Dumping the memory configuration, virtual processor register state and memory contents on a crash or unexpected VM Exit

To dump the details of the memory configuration, the virtual processors register state and the contents of the VM memory set the feature `dump_on_crash` and run a debug build. This will result in a dump file being created in the temporary directory. The name and location of the dump file will be printed to the console and logged as an error message.

There are no tools at this time to analyze the dump file, but it can be useful for debugging.