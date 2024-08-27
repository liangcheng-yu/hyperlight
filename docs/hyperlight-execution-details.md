# How code is run inside a VM

This document details how VMs are very quickly and efficiently created and configured to run arbitrary code.

## Background

Hyperlight is an SDK for creating micro virtual machines (VMs) intended for executing exactly one function. This use case is different from that of many other VM platforms, which are aimed at longer-running, more complex workloads.

As such, those platforms provide much more infrastructure of which running applications can take advantage. A very rough contrast between Hyperlight's offerings and other platforms is as follows:

| Feature                                                                 | Hyperlight | Other platforms    |
|-------------------------------------------------------------------------|------------|--------------------|
| Hardware isolation (vCPU, virtual memory)                               | Yes        | Yes                |
| Shared memory between host and in-VM process                            | Yes        | Yes <sup>[2]</sup> |
| Lightweight function calls between host and in-VM process (the "guest") | Yes        | No                 |
| Bootloader/OS kernel                                                    | No         | Yes <sup>[1]</sup> |
| Virtual networking                                                      | No         | Yes <sup>[2]</sup> |
| Virtual filesystem                                                      | No         | Yes <sup>[1]</sup> |


As seen in this table, Hyperlight offers little more than a CPU and memory. We've removed every feature we could, while still providing a machine on which arbitrary code can execute, so we can achieve our various use cases and efficiency targets.

## How code runs

With this background in mind, it's well worth focusing on the "lifecycle" of a VM -- how, exactly, a VM is created, modified, loaded, executed, and ultimately destroyed.

At the highest level, Hyperlight takes roughly the following steps to create and run arbitrary code inside a VM <sup>3</sup>:

1. Load arbitrary binary data as a [Portable Executable (PE)](https://en.wikipedia.org/wiki/Portable_Executable) file (either by manually parsing or optionally calling [`LoadLibraryA`](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibrarya) on Windows)
2. Using `mmap` (on Linux) or `VirtualAlloc` (on Windows) to create a shared memory region for the VM, then writing a "memory layout" with space to store a heap, stack, guest->host function calls, host->guest function calls, and more
3. Create an individual hypervisor instance ("partition" hereafter)
4. Create a single memory region within the partition, mapped to the shared memory created previously
5. Create one virtual CPU (vCPU) within the newly created partition
6. Write appropriate values to registers on the new vCPU, including the stack pointer (RSP), instruction pointer (RIP), control registers (CR0, CR1, etc...), and more
7. In a loop, tell previously created vCPU to run until we reach a halt message, one of several known error states (e.g. unmapped memory access), or an unsupported message
   1. In the former case, exit successfully
   2. In any of the latter cases, exit with a failure message

---

_<sup>[1]</sup> nearly universal support_

_<sup>[2]</sup> varied support_

_<sup>[3]</sup> since Hyperlight supports multiple hypervisor technologies, we've used generic wording to represent concepts common to all three, and specified instances in which only a subset of supported hypervisors support a feature_
