# simpleguest in rs

This is simpleguest, but written in rust. Here's how to build it:

```
cargo build --release --target x86_64-pc-windows-msvc

link.exe /NOLOGO /NXCOMPAT /SAFESEH:NO /RELEASE /SUBSYSTEM:NATIVE .\target\x86_64-pc-windows-msvc\release\libsimpleguest.rlib /LIBPATH:"C:\Users\danbugs\repos\hyperlight\x64\debug\" /OUT:"C:\Users\danbugs\repos\hyperlight\src\tests\Guests\simpleguest\x64\debug\simpleguest_new.exe" /ERRORREPORT:NONE /ALIGN:512 /NODEFAULTLIB /HEAP:"131072,131072" /DYNAMICBASE "HyperlightGuest.lib" /STACK:"65536,65536" /MACHINE:X64
```