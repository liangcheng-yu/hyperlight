# Hyperlight technical requirements document (TRD) 

In this technical requirements document (TRD), we have the following goals:

- Describe the high-level architecture of Hyperlight
- Provide relevant implementation details
- Provide additional information necessary for assessing the security and threat model of Hyperlight
- Detail the security claims Hyperlight makes

## High-level architecture

At a high level, Hyperlight's architecture is relatively simple. It consists of two primary components:

- Host SDK: the code that does the following:
  - Creates the Hyperlight VM, called the "sandbox"
  - Configures the VM, vCPU, and virtual registers
  - Configures VM memory
  - Loads the guest binary (see subsequent bullet point) into VM memory
  - Marshals calls to functions (called "guest functions") in the Guest binary inside the VM
  - Dispatches callbacks, called "host functions", from the guest back into the host
- Guest binary: the code that runs inside the Hyperlight sandbox and does the following:
  - Dispatches calls from the host into particular functions inside the guest
  - Marshals calls to host functions
  - _Drives user-defined logic_, often inside a language runtime or Wasm
    - We expect a Web Assembly (Wasm) runner to be our first production-ready guest. See the [hyperlight-wasm repository](https://github.com/deislabs/hyperlight-wasm) for more details.

## Relevant implementation details

As indicated in the previous "architecture" section, the two main components, the host and guest, interact in a specific, controlled manner. This section details the guest and host, and focuses on the details the implementation of that interaction.

### Guest binaries

Until this point, we've been using "guest" as an abstract term to indicate some binary to be run inside a Hyperlight sandbox. Because Hyperlight sandboxes only provide a limited set of functionality, guests must be compiled against and linked to all APIs necessary for providing the functionality above. These APIs are provided exclusively by our Hyperlight Guest library (currently written in C, but we plan to rewrite in Rust). 

>While guests may compile against additional libraries (e.g. `libc`), they are not guaranteed to run inside a sandbox, and likely won't.

For the remainder of this implementation section, we'll focus on the guest binary in [hyperlight-wasm](https://github.com/deislabs/hyperlight-wasm) so we can provide concrete examples.

The purpose of the hyperlight-wasm guest binary is to provide a Web Assembly (Wasm) runtime to execute a user-provided Wasm binary code. This guest binary is written in C, runs entirely inside the Hyperlight sandbox, and is linked with the Hyperlight Guest library to gain access to simple systems functionality like a `malloc` implementation and the ability to call host functions. Further, hyperlight-wasm and all other guests must be compiled and linked in a specific manner to run inside a Hyperlight sandbox. In these ways, the hyperlight sandbox roughly resembles the semantics of unikernel technologies.

>We have tentative plans to rewrite the Hyperlight Guest Library, and all our production-level guests including hyperlight-wasm, in Rust, so we can utilize higher-level abstractions within the guest and share more code between host and guest

The Hyperlight sandbox deliberately provides a very limited set of functionality to guest binaries (including hyperlight-wasm). We expect the most useful guests will execute code inside language interpreters or bytecode-level virtual machines, including Wasm VMs (we use [WAMR](https://github.com/bytecodealliance/wasm-micro-runtime) in hyperlight-wasm). Via this abstraction, we aim to provide functionality the "raw" Hyperlight sandbox does not provide directly. Any further functionality a given guest cannot provide can be provided via host functions.

### Host SDK

The Hyperlight host SDK provides a safe, robust, and secure Rust-native API for its users to create and interact with Hyperlight sandboxes. Due to (1) the nature of this project (see the section below on threat modeling for details), and (2) the fact the host SDK has access to host system resources, we have spent considerable time and energy ensuring the host SDK has two major features:

- It is memory safe
- It provides a public API that prevents its users from doing unsafe things, using Rust features and other techniques

>Note: the Host SDK also provides bindings to a C# API, but we do not currently have the resourcing to consider that API at the moment. We do have plans to improve this API and expand our set of bindings to other languages.

### Host-to-guest and guest-to-host communication

Communication between host and guest is done simply by configuring the relevant vCPU's instruction pointer, serializing the argument list, and executing the vCPU until a halt intercept is received. Guest-to-host communication is facilitated via the `outb` instruction, which is intercepted by the host and used to request functionality from the host (e.g. logging, calling functions).

More detail on this bidirectional communication mechanism can be found in [security.md](./security.md).

## Security threat model and guarantees 

The set of security guarantees we aim to provide with Hyperlight are as follows:

- All user-level code will, in production builds, be executed within a Hyperlight sandbox
- All Hyperlight sandboxes, in production builds, will be isolated from each other and the host using hypervisor provided Virtual Machines
- Guest binaries, in production Hyperlight builds, will have no access to the host system beyond VM-mapped memory (e.g. memory the host creates and maps into the system-appropriate VM) and a Hypervisor-provided vCPU, specifically a guest cannot request access to additional memory from the host
- Only host functions that are explicitly made available by the host to a guest are available to the Guest, the default state is that the guest has no access to host provided functions. 
- If a host provides a guest with such a host function, the guest will never be able to call that host function without explicitly being initialized first
  - In other words, a guest function must first be called before it can call a host function
- If a host provides a guest with a host function, the guest will never be able to execute that host function with an argument list of length and types not expected by the host
