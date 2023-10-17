.PHONY: build-linux
build-linux:
	@echo "Building simpleguest"
	@cargo build --release --target x86_64-pc-windows-gnu

.PHONY: build-windows
build-windows:
	@echo "Building simpleguest"
	@cargo build --release --target x86_64-pc-windows-msvc

.PHONY: link-linux
link-linux:
	@echo "Linking simpleguest"
	@lld-link-10 /nologo /nxcompat /safeseh:no /entry:entryPoint /subsystem:native ./target/x86_64-pc-windows-gnu/release/libsimpleguest.rlib /libpath:../../../../x64/debug /OUT:../../Guests/simpleguest/x64/debug/simpleguest_new.exe  /ERRORREPORT:NONE /align:512 /nodefaultlib /heap:131072,131072 /dynamicbase "HyperlightGuest.lib" /stack:65536,65536 /machine:x64

.PHONY: link-windows
link-windows:
	@echo "Linking simpleguest"
	@link.exe /NOLOGO /NXCOMPAT /SAFESEH:NO /RELEASE /ENTRY:"entryPoint" /SUBSYSTEM:NATIVE .\target\x86_64-pc-windows-msvc\release\libsimpleguest.rlib /LIBPATH:"..\..\..\..\x64\debug" /OUT:"..\..\Guests\simpleguest\x64\debug\simpleguest_new.exe" /ERRORREPORT:NONE /ALIGN:512 /NODEFAULTLIB /HEAP:"131072,131072" /DYNAMICBASE "HyperlightGuest.lib" /STACK:"65536,65536" /MACHINE:X64

.PHONY: update-linux
update-linux:
	@echo "Updating simpleguest binary"
	@mv ../../Guests/simpleguest/x64/debug/simpleguest_new.exe ../../Hyperlight.Tests/bin/debug/net6.0/simpleguest.exe

.PHONY: update-windows
update-windows:
	@echo "Updating simpleguest binary"
	@del /F /Q ..\..\Hyperlight.Tests\bin\debug\net6.0\simpleguest.exe
	@move /Y ..\..\Guests\simpleguest\x64\debug\simpleguest_new.exe ..\..\Hyperlight.Tests\bin\debug\net6.0\simpleguest.exe
