mkdir := if os() == "windows" { "mkdir -f -p" } else { "mkdir -p"} 

build-c-guests target=default-target: (assemble-asm target) (build-c-lib target) (link-c-lib target) (build-c-simpleguest target) (build-c-callbackguest target)

c-guest-options := '/GS /W3 /Zi /Od /fp:precise /WX- /std:c17  /showIncludes /MT /EHsc /nologo /diagnostics:column'

c-guest-defines := '/D "hidden=" /D "_MBCS" /D "GUEST_STACK_SIZE=65536" /D "GUEST_HEAP_SIZE=131072"'

c-guest-linker-options := '/MANIFEST:NO /NXCOMPAT /HEAP:131072,131072 /STACK:65536,65536 /DEBUG /RELEASE /ENTRY:"entryPoint" /ALIGN:4096 /FILEALIGN:4096 /NODEFAULTLIB /SAFESEH:NO /driver /SUBSYSTEM:NATIVE /MACHINE:x64 /DYNAMICBASE /TSAWARE:no /section:.text,ERP /section:.rdata,RP /section:.data,RWP /section:.pdata,RP'

compile-c-simpleguest target=default-target:
    cd src/tests/Guests/simpleguest && {{mkdir}} x64/{{target}} && clang-cl \
    main.c \
    /I"{{justfile_directory()}}/src/HyperlightGuest/include" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/third_party/printf" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/include/flatbuffers/generated" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/third_party/flatcc/include" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/third_party/libc/musl/arch/x86_64" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/third_party/libc/musl/include" \
    {{c-guest-options}} \
    {{c-guest-defines}}  \
    -Wno-error=implicit-function-declaration \
    /Fa"./x64/{{target}}/" /Fo"./x64/{{target}}/" /c -v 

link-c-simpleguest target=default-target:
    cd src/tests/Guests/simpleguest && lld-link /OUT:"{{justfile_directory()}}/src/tests/Guests/simpleguest/x64/{{target}}/simpleguest.exe" \
    {{c-guest-linker-options}} \
    /PDB:"./x64/{{target}}/simpleguest.pdb" \
    x64/{{target}}/main.obj  \
    {{justfile_directory()}}/x64/{{target}}/HyperlightGuest.lib

build-c-simpleguest target=default-target: (compile-c-simpleguest target) (link-c-simpleguest target)

compile-c-callbackguest target=default-target:
    cd src/tests/Guests/callbackguest && {{mkdir}} x64/{{target}} && clang-cl \
    main.c \
    /I"{{justfile_directory()}}/src/HyperlightGuest/include" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/third_party/printf" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/include/flatbuffers/generated" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/third_party/flatcc/include" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/third_party/libc/musl/arch/x86_64" \
    /I"{{justfile_directory()}}/src/HyperlightGuest/third_party/libc/musl/include" \
    {{c-guest-options}} \
    {{c-guest-defines}} \
    -Wno-error=implicit-function-declaration \
    /Fa"x64/{{target}}/" /Fo"x64/{{target}}/" /c -v

link-c-callbackguest target=default-target:
    cd src/tests/Guests/callbackguest && lld-link /OUT:"{{justfile_directory()}}/src/tests/Guests/callbackguest/x64/{{target}}/callbackguest.exe" \
    {{c-guest-linker-options}} \
    /PDB:"./x64/{{target}}/callbackguest.pdb" \
    x64/{{target}}/main.obj \
    {{justfile_directory()}}/x64/{{target}}/HyperlightGuest.lib

build-c-callbackguest target=default-target: (compile-c-callbackguest target) (link-c-callbackguest target)

c-lib-includes := '/I"./third_party/libc/musl/src/include" /I"./third_party/libc/musl/include" /I"./third_party/libc/musl/src/internal" /I"./third_party/libc/musl/arch/generic" /I"./third_party/libc/musl/arch/x86_64" /I"./third_party/dlmalloc" /I"./include" /I"./third_party/flatcc/include" /I"./third_party/printf" /I"./include/flatbuffers/generated"'

c-lib-sources := './src/HyperlightGuest.c'

dlmalloc-sources := './third_party/dlmalloc/malloc.c'

flatcc-sources := './third_party/flatcc/src/runtime/builder.c ./third_party/flatcc/src/runtime/emitter.c ./third_party/flatcc/src/runtime/refmap.c'

###
### Note: Keep an extra space after the last entry in each of the variables so the aggregate variable libc-sources expands properly!
###

libc-musl-ctype-sources := './third_party/libc/musl/src/ctype/isalnum.c ./third_party/libc/musl/src/ctype/isalpha.c ./third_party/libc/musl/src/ctype/isdigit.c ./third_party/libc/musl/src/ctype/isgraph.c ./third_party/libc/musl/src/ctype/islower.c ./third_party/libc/musl/src/ctype/isprint.c ./third_party/libc/musl/src/ctype/isspace.c ./third_party/libc/musl/src/ctype/isupper.c ./third_party/libc/musl/src/ctype/isxdigit.c ./third_party/libc/musl/src/ctype/tolower.c ./third_party/libc/musl/src/ctype/toupper.c '

libc-musl-error-sources := './third_party/libc/musl/src/errno/__errno_location.c '

libc-musl-internal-sources := 'third_party/libc/musl/src/internal/floatscan.c ./third_party/libc/musl/src/internal/intscan.c ./third_party/libc/musl/src/internal/shgetc.c '

libc-musl-math-sources := './third_party/libc/musl/src/math/copysign.c ./third_party/libc/musl/src/math/copysignl.c ./third_party/libc/musl/src/math/fabs.c ./third_party/libc/musl/src/math/fabsl.c ./third_party/libc/musl/src/math/fmod.c ./third_party/libc/musl/src/math/fmodl.c ./third_party/libc/musl/src/math/scalbnl.c '

libc-musl-stdio-sources := './third_party/libc/musl/src/stdio/__toread.c ./third_party/libc/musl/src/stdio/__uflow.c '

libc-musl-stdlib-sources := './third_party/libc/musl/src/stdlib/atoi.c ./third_party/libc/musl/src/stdlib/strtod.c ./third_party/libc/musl/src/stdlib/strtol.c '

libc-musl-string-sources := './third_party/libc/musl/src/string/memchr.c ./third_party/libc/musl/src/string/memcmp.c ./third_party/libc/musl/src/string/memcpy.c ./third_party/libc/musl/src/string/memmove.c ./third_party/libc/musl/src/string/memset.c ./third_party/libc/musl/src/string/stpncpy.c ./third_party/libc/musl/src/string/strchr.c ./third_party/libc/musl/src/string/strchrnul.c ./third_party/libc/musl/src/string/strcmp.c ./third_party/libc/musl/src/string/strcspn.c ./third_party/libc/musl/src/string/strlen.c ./third_party/libc/musl/src/string/strncasecmp.c ./third_party/libc/musl/src/string/strncat.c ./third_party/libc/musl/src/string/strncmp.c ./third_party/libc/musl/src/string/strncpy.c ./third_party/libc/musl/src/string/strspn.c ./third_party/libc/musl/src/string/strstr.c '

libc-sources := libc-musl-ctype-sources + libc-musl-error-sources + libc-musl-internal-sources + libc-musl-math-sources + libc-musl-stdio-sources + libc-musl-stdlib-sources + libc-musl-string-sources

printf-sources := './third_party/printf/printf.c'

c-lib-defines := '/D "HAVE_MMAP=0" /D "HAVE_MORECORE=1" /D "MORECORE=hyperlightMoreCore" /D "MORECORE_CANNOT_TRIM=1" /D "MORECORE_CONTIGUOUS=1" /D "USE_DL_PREFIX" /D "NO_MALLOC_STATS" /D "ABORT=dlmalloc_abort()" /D "HYPERLIGHT" /D "HYPERLIGHT_PAGESIZE=4096" /D "HYPERLIGHT_GRANULARITYSIZE=65536" /D "__x86_64__" /D "LACKS_SYS_TYPES_H" /D "LACKS_UNISTD_H" /D "__DEFINED_va_list" /D "__DEFINED___isoc_va_list" /D "hidden=" /D "MMAP_CLEARS=0" /D "__FLT_EVAL_METHOD__=1" /D "__x86_64__" /D "__LITTLE_ENDIAN__" /D "calloc=dlcalloc" /D "free=dlfree" /D "malloc=dlmalloc" /D "realloc=dlrealloc" /D "FLATCC_USE_GENERIC_ALIGNED_ALLOC=0" /D "MALLOC_FAILURE_ACTION=dlmalloc_failure()" /D "_UNICODE" /D "UNICODE" /D "weak_alias(old, new) = "'

c-lib-flags-debug := '/MDd /Od /Ob0 /Z7 /LDd'

c-lib-flags-release := '/MD /O2 /Gy /LD' 

c-lib-flags-common := '/std:c17 /WX- -Wno-error=implicit-function-declaration /W3 /GS /fp:precise /Gd'

assemble-asm target=default-target:
    cd src/HyperlightGuest && {{mkdir}} x64/{{target}} && \
    llvm-ml -m64 /c /nologo /Fo "x64/{{target}}/__chkstk.obj" /Ta "src/asm/__chkstk.asm" && \
    llvm-ml -m64 /c /nologo /Fo "x64/{{target}}/__security_cookie.obj" /Ta "src/asm/__security_cookie.asm" && \
    llvm-ml -m64 /c /nologo /Fo "x64/{{target}}/jmp.obj" /Ta "src/asm/jmp.asm" && \
    llvm-ml -m64 /c /nologo /Fo "x64/{{target}}/outb.obj" /Ta "src/asm/outb.asm" && \
    llvm-ml -m64 /c /nologo /Fo "x64/{{target}}/regaccess.obj" /Ta "src/asm/regaccess.asm" && \
    llvm-ml -m64 /c /nologo /Fo "x64/{{target}}/__halt.obj" /Ta "src/asm/__halt.asm" 

build-c-lib target=default-target:
    cd src/HyperlightGuest && {{mkdir}} x64/{{target}} && clang-cl \
    {{c-lib-sources}} \
    {{dlmalloc-sources}} \
    {{flatcc-sources}} \
    {{libc-sources}} \
    {{printf-sources}} \
    {{c-lib-includes}} \
    {{c-lib-defines}} \
    {{c-lib-flags-common}} \
    {{ if target == "debug" { c-lib-flags-debug } else { c-lib-flags-release } }} \
    /Fa"x64/{{target}}/" /nologo /Fo"x64/{{target}}/" /diagnostics:column /X -v /c

link-c-lib target=default-target:
    cd src/HyperlightGuest && {{mkdir}} ../../x64/{{target}} && \
    llvm-lib /MACHINE:X64 /NOLOGO {{ if target == "debug" { "" } else { "/LTCG" } }} \
        /OUT:"../../x64/{{target}}/HyperlightGuest.lib" \
        x64/{{target}}/HyperlightGuest.obj \
        x64/{{target}}/malloc.obj \
        x64/{{target}}/printf.obj \
        x64/{{target}}/builder.obj \
        x64/{{target}}/emitter.obj \
        x64/{{target}}/refmap.obj \
        x64/{{target}}/isalnum.obj \
        x64/{{target}}/isalpha.obj \
        x64/{{target}}/isdigit.obj \
        x64/{{target}}/isgraph.obj \
        x64/{{target}}/islower.obj \
        x64/{{target}}/isprint.obj \
        x64/{{target}}/isspace.obj \
        x64/{{target}}/isupper.obj \
        x64/{{target}}/isxdigit.obj \
        x64/{{target}}/tolower.obj \
        x64/{{target}}/toupper.obj \
        x64/{{target}}/__errno_location.obj \
        x64/{{target}}/floatscan.obj \
        x64/{{target}}/intscan.obj \
        x64/{{target}}/shgetc.obj \
        x64/{{target}}/copysign.obj \
        x64/{{target}}/copysignl.obj \
        x64/{{target}}/fabs.obj \
        x64/{{target}}/fabsl.obj \
        x64/{{target}}/fmod.obj \
        x64/{{target}}/fmodl.obj \
        x64/{{target}}/scalbnl.obj \
        x64/{{target}}/__toread.obj \
        x64/{{target}}/__uflow.obj \
        x64/{{target}}/atoi.obj \
        x64/{{target}}/strtod.obj \
        x64/{{target}}/strtol.obj \
        x64/{{target}}/memchr.obj \
        x64/{{target}}/memcmp.obj \
        x64/{{target}}/memcpy.obj \
        x64/{{target}}/memmove.obj \
        x64/{{target}}/memset.obj \
        x64/{{target}}/stpncpy.obj \
        x64/{{target}}/strchr.obj \
        x64/{{target}}/strchrnul.obj \
        x64/{{target}}/strcmp.obj \
        x64/{{target}}/strcspn.obj \
        x64/{{target}}/strlen.obj \
        x64/{{target}}/strncasecmp.obj \
        x64/{{target}}/strncat.obj \
        x64/{{target}}/strncmp.obj \
        x64/{{target}}/strncpy.obj \
        x64/{{target}}/strspn.obj \
        x64/{{target}}/strstr.obj \
        x64/{{target}}/jmp.obj \
        x64/{{target}}/regaccess.obj \
        x64/{{target}}/__chkstk.obj \
        x64/{{target}}/__security_cookie.obj \
        x64/{{target}}/__halt.obj \
        x64/{{target}}/outb.obj