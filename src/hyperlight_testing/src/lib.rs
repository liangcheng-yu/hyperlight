// This crate contains testing utilities which need to be shared across multiple
// crates in this project.
use anyhow::{anyhow, Result};
use hex_literal::hex;
use std::path::PathBuf;

pub const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// Join all the `&str`s in the `v` parameter as a path with appropriate
/// path separators, then prefix it with `start`, again with the appropriate
/// path separator
fn join_to_path(start: &str, v: Vec<&str>) -> PathBuf {
    let fold_start: PathBuf = {
        let mut pb = PathBuf::new();
        pb.push(start);
        pb
    };
    let fold_closure = |mut agg: PathBuf, cur: &&str| {
        agg.push(cur);
        agg
    };
    v.iter().fold(fold_start, fold_closure)
}

/// Get a new `PathBuf` pointing to `callbackguest.exe`
pub fn callback_guest_as_pathbuf() -> PathBuf {
    let build_dir_selector = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    // $REPO_ROOT/src/tests/Guests/callbackguest/x64/{debug,release}/callbackguest.exe"
    join_to_path(
        MANIFEST_DIR,
        vec![
            "..",
            "tests",
            "Guests",
            "callbackguest",
            "x64",
            build_dir_selector,
            "callbackguest.exe",
        ],
    )
}

/// Get a fully qualified OS-specific path to the callbackguest.exe
/// binary. Convenience method for calling `callback_guest_as_pathbuf`, then
/// converting the result into an owned `String`
pub fn callback_guest_as_string() -> Result<String> {
    let buf = callback_guest_as_pathbuf();
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert callback guest PathBuf to string"))
}

/// Get a new `PathBuf` to a specified Rust guest
/// $REPO_ROOT/src/tests/rust_guests/bin/${profile}/net6.0
pub fn rust_guest_as_pathbuf(guest: &str) -> PathBuf {
    let build_dir_selector = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    join_to_path(
        MANIFEST_DIR,
        vec![
            "..",
            "tests",
            "rust_guests",
            "bin",
            build_dir_selector,
            format!("{}.exe", guest).as_str(),
        ],
    )
}

/// Get a fully qualified OS-specific path to the simpleguest.exe
/// binary. Convenience method for calling `simple_guest_as_pathbuf`, then
/// converting the result into an owned `String`
pub fn simple_guest_as_string() -> Result<String> {
    let buf = rust_guest_as_pathbuf("simpleguest");
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert simple guest PathBuf to string"))
}

/// Get a fully qualified OS-specific path to the dummyguest.exe
/// binary. Convenience method for calling `dummy_guest_as_pathbuf`, then converting
/// the result into an owned `String`
pub fn dummy_guest_as_string() -> Result<String> {
    let buf = rust_guest_as_pathbuf("dummyguest");
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert dummy guest PathBuf to string"))
}

// The test data is a valid flatbuffers buffer representing a guestfunction call as follows:
// int PrintSevenArgs(string="Test7", int=8, long=9, string="Tested", string="Test7", bool=false, bool=true)
pub fn get_guest_function_call_test_data() -> Vec<u8> {
    hex!("34010000140000000000000000000a00100008000c0007000a00000000000001040100000400000007000000d0000000b000000084000000600000003c000000240000000400000054ffffff000000040c000000000006000800070006000000000000018cffffff00000004080000000400040004000000a0ffffff00000003040000007affffff04000000050000005465737437000000c0ffffff00000003040000009affffff04000000060000005465737465640000c4ffffff000000020c000000000006000c00040006000000090000000000000008000c0007000800080000000000000104000000e2ffffff0800000008000e000700080008000000000000030c000000000006000800040006000000040000000500000054657374370000000e0000005072696e74536576656e417267730000").to_vec()
}

// The test data is a valid flatbuffers buffer representing a host function call as follows:
// int HostMethod1(string="Hello from GuestFunction1, Hello from CallbackTest")
pub fn get_host_function_call_test_data() -> Vec<u8> {
    hex!("940000001000000000000a00100008000c0007000a000000000000026c00000004000000010000000c00000008000e000700080008000000000000030c000000000006000800040006000000040000003200000048656c6c6f2066726f6d20477565737446756e6374696f6e312c2048656c6c6f2066726f6d2043616c6c6261636b5465737400000b000000486f73744d6574686f643100").to_vec()
}
