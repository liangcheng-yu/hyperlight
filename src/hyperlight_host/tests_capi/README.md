# `hyperlight-host` C API Tests

This directory has a collection of tests for `hyperlight-host`'s C API. Run tests using the `Justfile` in the parent directory:

```shell
just run-tests-capi
```

The tests depend on the [µnit test framework](https://nemequ.github.io/munit/), and `µnit` is installed in this directory as a [submodule](https://git-scm.com/book/en/v2/Git-Tools-Submodules) for convenience. When you check out this repository for the first time, you have to run the following two commands (within this directory) right after `git clone` to properly download the `µnit` code:

```shell
git submodule init
git submodule update
```

## Memory Correctness Tests

We use [Valgrind](https://valgrind.org/) on Linux to help check for memory correctness. You'll need to ensure you have Valgrind version 3.19.0 or higher installed on your system before you run these tests. There is an `install_valgrind.sh` script in the `build` directory at the root of this repository. It installs Valgrind 3.19.0 on your system.

>If you're on Ubuntu or Debian, the default Apt repository will install Valgrind version 3.15.0, which is too old.

Once you have Valgrind installed, run the memory correctness tests with the below command, from the parent directory (`src/hyperlight_host`):

```shell
just valgrind-tests-capi
```
