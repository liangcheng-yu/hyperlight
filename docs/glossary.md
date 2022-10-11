# Glossary

* [Hyperlight](#hyperlight)
* [Hyperlight SDK](#hyperlight-sdk)
* [Calling Application](#calling-application)
* [Host](#host)
* [Hypervisor](#hypervisor)
* [Driver](#driver)
* [Hyper-V](#hyper-v)
* [KVM](#kvm)
* [Guest](#guest)
* [Partition](#partition)
* [Workload](#workload)

## Hyperlight

Hyperlight refers to the Hyperlight Project and not a specific component.

Hyperlight is intended to be used as a [library or SDK](#hyperlight-sdk) by a calling application, and is not a long-running process or service that must be installed on a [host](#host).

<!-- TODO: Uncomment when we have this page committed -->
<!--See the [Hyperlight Architecture](./architecture.md) overview for more details.-->

## Hyperlight SDK

The Hyperlight SDK, often referred to as simply the SDK, is a set of language-specific libraries that enable a [calling application](#calling-application) to run isolated [workloads](#workload).

## Calling Application

Hyperlight is a library that is called by another application, known as the "calling application".
The calling application runs on the [host](#host) and is responsible for determining the [guest](#guest) and [workloads](#workload) to execute.

## Host

Host is the machine upon which the calling application and [hypervisor](#hypervisor) are running.
A host could be a bare metal or virtual machine.

## Hypervisor

Hypervisor is a virtual machine monitor (VMM) that is responsible for executing the [guest](#guest) in an isolated [partition](#partition).
Hyperlight has [drivers](#driver) the following hypervisors: [Hyper-V](#hyper-v) on Windows, [Hyper-V](#hyper-v) on Linux, and [KVM](#kvm).

## Driver

The Hyperlight SDK supports executing workloads on particular [hypervisors](#hypervisor) through drivers.
Each supported hypervisor has its own driver to manage interacting with that hypervisor.

## Hyper-V

Hyper-V is a [hypervisor](#hypervisor) capable of running isolated [partitions](#partition) on both Windows and Linux.

## KVM

Kernel-based Virtual Machine (KVM) is a [hypervisor](#hypervisor) capable of running isolated [partitions](#partition) on Linux.

## Guest

The guest is a binary that executes inside the hypervisor [partition](#partition).
Guests implement a limited set of functionality, similar to system calls or syscalls, that a workload can rely upon for critical functionality such as interacting with the host machine, such as printing output or allocating/de-allocating memory. that is responsible for interacting with the host and executing [workloads](#workload).

Having purpose-fit guests, as opposed to running a full operating system, is how Hyperlight achieves low-latency startup times of workloads.
Since it doesn't need to first boot an entire operating system before executing the workload.

The interface that a guest must implement is specific to the associated [host](#host) and the type of workloads that it may be specialized for executing, such as WebAssembly Modules (WASM), or a specific language.

## Partition

A partition is an execution environment managed by a hypervisor that isolates a [guest](#guest) from the [host](#host).
The hypervisor prevents the guest from directly accessing the host resources, such as the terminal, memory or CPU.
All resources are interacted with through an abstraction, allowing the hypervisor to limit the guest's access to the host.

Hyperlight does not call the isolated environment in which workloads execute a "virtual machine" because the set of capabilities provided to a guest are dramatically limited compared to that of a traditional virtual machine whose guest is a full operating system.
We use the term partition instead to indicate that it is isolated, while avoiding implying that it has the same capabilities as what most of are used to when discussing virtual machines.

## Workload

A workload is the code that the calling application wants to execute in an isolated [partition](#partition).
