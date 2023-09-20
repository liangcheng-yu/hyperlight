.PHONY: build
build-linux:
	@echo "Building simpleguest"
	@cargo build --release --target x86_64-pc-windows-gnu

.PHONY: link
link-linux:
	@echo "Linking simpleguest"
	@lld-link-10 /nologo /nxcompat /safeseh:no /entry:entryPoint /subsystem:native ./target/x86_64-pc-windows-gnu/release/libsimpleguest.rlib /libpath:../../../../x64/debug /OUT:../../Guests/simpleguest/x64/debug/simpleguest_new.exe  /ERRORREPORT:NONE /align:512 /nodefaultlib /heap:131072,131072 /dynamicbase "HyperlightGuest.lib" /stack:65536,65536 /machine:x64

.PHONY: update
update:
	@echo "Updating simpleguest binary"
	mv ../../Guests/simpleguest/x64/debug/simpleguest_new.exe ../../Hyperlight.Tests/bin/debug/net6.0/simpleguest.exe
