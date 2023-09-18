# simpleguest in rs

This is simpleguest, but written in rust. Here's how to build it:

```sh
# on Windows:
cargo build --release --target x86_64-pc-windows-msvc

link.exe /NOLOGO /NXCOMPAT /SAFESEH:NO /RELEASE /ENTRY:"entryPoint" /SUBSYSTEM:NATIVE .\target\x86_64-pc-windows-msvc\release\libsimpleguest.rlib /LIBPATH:"C:\Users\danbugs\repos\hyperlight\x64\debug\" /OUT:"C:\Users\danbugs\repos\hyperlight\src\tests\Guests\simpleguest\x64\debug\simpleguest_new.exe" /ERRORREPORT:NONE /ALIGN:512 /NODEFAULTLIB /HEAP:"131072,131072" /DYNAMICBASE "HyperlightGuest.lib" /STACK:"65536,65536" /MACHINE:X64

# on Linux:
cargo build --release --target x86_64-pc-windows-gnu

lld-link-10  /nxcompat /safeseh:no /entry:entryPoint /subsystem:native ./target/x86_64-pc-windows-gnu/release/libsimpleguest.rlib /libpath:../../../../x64/debug /OUT:../../Guests/simpleguest/x64/debug/simpleguest_new.exe  /align:512 /nodefaultlib /heap:131072,131072 /dynamicbase "HyperlightGuest.lib" /stack:65536,65536 /machine:x64
```