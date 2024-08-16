fn main() {
    println!("cargo:rerun-if-changed=third_party");
    println!("cargo:rerun-if-changed=src/alloca");

    let mut cfg = cc::Build::new();

    if cfg!(feature = "printf") {
        cfg.include("third_party/printf")
            .file("third_party/printf/printf.c");
    }

    if cfg!(feature = "libc") {
        cfg.file("third_party/libc/musl/src/ctype/isalpha.c")
            .file("third_party/libc/musl/src/ctype/isalnum.c")
            .file("third_party/libc/musl/src/ctype/isdigit.c")
            .file("third_party/libc/musl/src/ctype/isgraph.c")
            .file("third_party/libc/musl/src/ctype/islower.c")
            .file("third_party/libc/musl/src/ctype/isprint.c")
            .file("third_party/libc/musl/src/ctype/isspace.c")
            .file("third_party/libc/musl/src/ctype/isupper.c")
            .file("third_party/libc/musl/src/ctype/isxdigit.c")
            .file("third_party/libc/musl/src/ctype/tolower.c")
            .file("third_party/libc/musl/src/ctype/toupper.c")
            .file("third_party/libc/musl/src/errno/__errno_location.c")
            .file("third_party/libc/musl/src/internal/floatscan.c")
            .file("third_party/libc/musl/src/internal/intscan.c")
            .file("third_party/libc/musl/src/internal/shgetc.c")
            .file("third_party/libc/musl/src/math/copysign.c")
            .file("third_party/libc/musl/src/math/copysignl.c")
            .file("third_party/libc/musl/src/math/fabs.c")
            .file("third_party/libc/musl/src/math/fabsl.c")
            .file("third_party/libc/musl/src/math/fmod.c")
            .file("third_party/libc/musl/src/math/fmodl.c")
            .file("third_party/libc/musl/src/math/scalbnl.c")
            .file("third_party/libc/musl/src/math/__signbit.c")
            .file("third_party/libc/musl/src/math/__signbitl.c")
            .file("third_party/libc/musl/src/math/__fpclassify.c")
            .file("third_party/libc/musl/src/math/__fpclassifyl.c")
            .file("third_party/libc/musl/src/stdio/__toread.c")
            .file("third_party/libc/musl/src/stdio/__uflow.c")
            .file("third_party/libc/musl/src/stdlib/atoi.c")
            .file("third_party/libc/musl/src/stdlib/strtod.c")
            .file("third_party/libc/musl/src/stdlib/strtol.c")
            .file("third_party/libc/musl/src/stdlib/qsort.c")
            .file("third_party/libc/musl/src/stdlib/qsort_nr.c")
            .file("third_party/libc/musl/src/stdlib/bsearch.c")
            .file("third_party/libc/musl/src/string/memchr.c")
            .file("third_party/libc/musl/src/string/memcmp.c")
            .file("third_party/libc/musl/src/string/memcpy.c")
            .file("third_party/libc/musl/src/string/memmove.c")
            .file("third_party/libc/musl/src/string/memset.c")
            .file("third_party/libc/musl/src/string/stpncpy.c")
            .file("third_party/libc/musl/src/string/strchr.c")
            .file("third_party/libc/musl/src/string/strchrnul.c")
            .file("third_party/libc/musl/src/string/strcmp.c")
            .file("third_party/libc/musl/src/string/strcspn.c")
            .file("third_party/libc/musl/src/string/strlen.c")
            .file("third_party/libc/musl/src/string/strncasecmp.c")
            .file("third_party/libc/musl/src/string/strncat.c")
            .file("third_party/libc/musl/src/string/strncmp.c")
            .file("third_party/libc/musl/src/string/strncpy.c")
            .file("third_party/libc/musl/src/string/strspn.c")
            .file("third_party/libc/musl/src/string/strstr.c")
            .file("third_party/libc/musl/src/prng/rand.c")
            .include("third_party/libc/musl/src/include")
            .include("third_party/libc/musl/include")
            .include("third_party/libc/musl/src/internal")
            .include("third_party/libc/musl/arch/generic")
            .include("third_party/libc/musl/arch/x86_64");
    }

    if cfg!(feature = "alloca") {
        cfg.file("src/alloca/alloca.c")
            .define("_alloca", "_alloca_wrapper")
            .flag("-Wno-return-stack-address");
    }

    if cfg!(any(
        feature = "printf",
        feature = "libc",
        feature = "alloca"
    )) {
        cfg.define("hidden", "");
        cfg.define("__DEFINED_va_list", None);
        cfg.define("__DEFINED___isoc_va_list", None);
        cfg.define("__x86_64__", None);
        cfg.define("__LITTLE_ENDIAN__", None);

        cfg.define("malloc", "hlmalloc");
        cfg.define("calloc", "hlcalloc");
        cfg.define("free", "hlfree");
        cfg.define("realloc", "hlrealloc");
        cfg.define("weak_alias(old, new) ", " ");

        // silence compiler warnings
        cfg.flag("-Wno-sign-compare");
        cfg.flag("-Wno-bitwise-op-parentheses");
        cfg.flag("-Wno-unknown-pragmas");
        cfg.flag("-Wno-shift-op-parentheses");
        cfg.flag("-Wno-logical-op-parentheses");

        cfg.compiler("clang-cl");

        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                std::env::set_var("AR_x86_64_pc_windows_msvc", "llvm-lib");
            }
        }

        cfg.compile("hyperlight_guest");
    }
}
