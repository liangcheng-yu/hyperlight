# The "dummy" guest in Rust

This is the dummy guest, written in Rust. The purpose of this guest binary is only for use with Hyperlight tests. It utilizes no Rust functionality and allocates no memory, so it's useful for a wide range of lower-level tests.

The command to build this guest, which you must run on windows (to get access to the linker `link.exe`) is as follows:

```
cargo build --release --target x86_64-pc-windows-msvc

link.exe /NOLOGO /NXCOMPAT /SAFESEH:NO /RELEASE /ENTRY:"entryPoint" /SUBSYSTEM:NATIVE .\target\x86_64-pc-windows-msvc\release\libdummyguest.rlib /LIBPATH:"C:\Users\danbugs\repos\hyperlight\x64\debug\" /OUT:"C:\Users\danbugs\repos\hyperlight\src\tests\Guests\dummyguest\x64\debug\dummyguest_new.exe" /ERRORREPORT:NONE /ALIGN:512 /NODEFAULTLIB /HEAP:"131072,131072" /DYNAMICBASE /STACK:"65536,65536" /MACHINE:X64
```