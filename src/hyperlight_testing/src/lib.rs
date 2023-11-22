// This crate contains testing utilities which need to be shared across multiple
// crates in this project.
use anyhow::{anyhow, Result};
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

/// Get a new `PathBuf` pointing to the base of the test bin directory
/// $REPO_ROOT/src/testsHyperlight.Tests/bin/${profile}/net6.0
pub fn test_bin_base() -> PathBuf {
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
            "Hyperlight.Tests",
            "bin",
            build_dir_selector,
            "net6.0",
        ],
    )
}

/// Get a new `PathBuf` pointing to `callbackguest.exe`
pub fn callback_guest_buf() -> PathBuf {
    // $REPO_ROOT/src/tests/Hyperlight.Tests/bin/Debug/net6.0/callbackguest.exe"
    let mut base = test_bin_base();
    base.push("callbackguest.exe");
    base
}

/// Get a fully qualified OS-specific path to the callbackguest.exe
/// binary. Convenience method for calling `callback_guest_buf`, then
/// converting the result into an owned `String`
pub fn callback_guest_path() -> Result<String> {
    let buf = callback_guest_buf();
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert callback guest PathBuf to string"))
}

/// Get a new `PathBuf` pointing to `simpleguest.exe`
pub fn simple_guest_buf() -> PathBuf {
    // $REPO_ROOT/src/tests/Hyperlight.Tests/bin/debug/net6.0/simpleguest.exe"
    let mut base = test_bin_base();
    base.push("simpleguest.exe");
    base
}

/// Get a fully qualified OS-specific path to the simpleguest.exe
/// binary. Convenience method for calling `simple_guest_buf`, then
/// converting the result into an owned `String`
pub fn simple_guest_path() -> Result<String> {
    let buf = simple_guest_buf();
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert simple guest PathBuf to string"))
}
