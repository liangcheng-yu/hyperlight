# The "dummy" guest in Rust

This is dummyguest, but written in rust. Here's how to build it:

```
cargo build --release --target x86_64-pc-windows-msvc

link.exe /NOLOGO /NXCOMPAT /SAFESEH:NO /RELEASE /ENTRY:"entryPoint" /SUBSYSTEM:NATIVE .\target\x86_64-pc-windows-msvc\release\libdummyguest.rlib /LIBPATH:"C:\Users\danbugs\repos\hyperlight\x64\debug\" /OUT:"C:\Users\danbugs\repos\hyperlight\src\tests\Guests\dummyguest\x64\debug\dummyguest_new.exe" /ERRORREPORT:NONE /ALIGN:512 /NODEFAULTLIB /HEAP:"131072,131072" /DYNAMICBASE /STACK:"65536,65536" /MACHINE:X64
```