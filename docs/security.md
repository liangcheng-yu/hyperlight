# The security of Hyperlight

This document discusses the general security and specific mechanisms used in Hyperlight to isolate _guest code_ -- arbitrary, potentially-hostile, third-party code -- from _host code_ -- the software that uses Hyperlight itself to drive the guest code.

This document will have two areas of focus, as follows:

1. How virtual machines (VMs) are constructed and run
1. How the host and guest communicate

## Constructing VMs

Hyperlight provides a `Sandbox` API for executing guest code in a virtualized-hardware isolated environment. To achieve this isolation, every `Sandbox` corresponds to one virtual machine, complete with mapped memory (e.g. memory created with `mmap` on Linux or `VirtualAlloc` on Windows), a virtual CPU (vCPU), corresponding virtual register values to enable guest binaries to run, and nothing else.

Importantly, this `Sandbox` mechanism significantly restricts the capabilities offered to guest binaries. There is no underlying operating system (OS) running inside of Hyperlight `Sandbox`es, and thus the guest has no access to common facilities provided by an OS, including multiprocessing / time sharing, syscalls, and more. The facilities provided to guests are listed as follows:

- Call pre-authorized functions on the host
  - See the "communication" section below for details on this calling convention
- Access a finite memory space
  - Hyperlight sets up a rudimentary memory paging system, compatible with most `malloc` implementations, and provides in the guest a memory allocation/de-allocation API based on [`dlmalloc`](https://github.com/ennorehling/dlmalloc). This system is confined only to the functionality inside the VM, and involves no ongoing host interaction
  - If the guest attempts to access addresses outside this memory space, the host will be notified. Upon such a notification, Hyperlight immediately shuts down the guest.
- Run machine code on a virtual CPU (vCPU)

>Since guests run directly on the vCPU, they are pre-compiled against and linked to all APIs necsssary for providing the functionality above. While guests may compile against additional libraries (e.g. `libc`), they will fail to run when instantiated inside a `Sandbox`.

The limitations implied by the above list in turn imply the following important limitations of note:

- No host-provided page-faulting mechanism (e.g. classical virtual memory)
- No virtual file system
- No virtual networking stack
- Generally, no virtual device support whatsoever

## Guest-to-host and host-to-guest communication

An important feature in the list from the previous section was the ability for a guest to call a pre-arranged function on the host. This feature represents the largest attack surface area of this project, and it will be detailed in this section.

### Calling guest functions from the host

The mechanism for calling guest functions from the host is simpler and exposes a relatively minimal attack surface. To execute a function, the host does the following:

1. Sets the instruction pointer to the address of a "dispatch" function in the guest
2. Serializes the function name and its arguments into a pre-arranged, hard-coded location in shared memory
  - We have plans to restrict the read/write permissions of this and similar locations in the following section
3. Executes the vCPU until receiving a halt instruction
4. Reads the return value from a pre-arranged, hard-coded location in shared memory 
   - Similar to above, we have plans to restrict the read/write permissions of this and similar locations in the following section

When the guest's "dispatch" function (from #1 above) executes, it reads the function name and arguments from the shared memory, and then calls the appropriate function. When the executing function returns a value, the dispatch function in turn serializes that value to shared memory and halts.

#### Serialization and deserialization

Much of the opportunity for attacks (the "attack surface area", commonly) may come from the serialization/deserialization logic on the host. To help minimize this surface area, we avoid hand-writing any serialization logic and instead rely on [FlatBuffers](https://flatbuffers.dev) to do the following for us: (a) formally specify the data structures passed to/from the host and guest, and (b) generate serialization/deserialization code.

### Calling host functions from the guest

In addition to host-to-guest calls, the `Sandbox` also provides a mechanism for the host to register functions that may be called from the guest. This mechanism is useful to allow developers to provide guests with strictly controlled access to functionality we don't make available by default inside the VM. As indicated earlier, this mechanism likely represents the largest attack surface area of this project.

#### Implementation details

The implementation of this mechanism is similar to the host-to-guest mechanism, but with a few important differences:

1. The guest calls a "dispatch" function on the host by issuing an [`outb`](https://man7.org/linux/man-pages/man2/outb.2.html) vCPU interrupt
2. Upon receiving the `outb` interrupt, the host reads function name and arguments from a pre-arranged, hard-coded location in shared memory _different_ from the associated location in the previous host-to-guest section.
3. Similarly, when the host-native function returns a value to the host dispatch function from (1), the host dispatch function serializes the return value to a pre-arranged, hard-coded location in shared memory _different_ from the associated location in the previous host-to-guest section.

#### Attack surface area

We expect there are at least the following two primary attack vectors in this mechanism, but recognize there are likely others as well (e.g. side-channel attacks based on memory access patterns or vCPU execution time):

1. The `outb` and associated memory-sharing mechanism may be subject to abuse
2. Host-provided functions, which the host's dispatch function executes, are not audited or managed in any way

We believe the following mitigations may help ameliorate these attack vectors, but similarly expect there to be other ameliorations to these vectors and/or or strategies to obsolete them entirely:

- Restrict the read/write permissions of shared memory to help limit abuse of the `outb` mechanism in (1) above
- Run host-provided functions in a separate process, with limited permissions, to help limit the impact of the un-audited or un-managed functions from (2)
