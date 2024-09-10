// This crate contains testing utilities which need to be shared across multiple
// crates in this project.
use std::path::PathBuf;

use anyhow::{anyhow, Result};

pub const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
pub mod logger;
pub mod simplelogger;
pub mod tracing_subscriber;

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
/// binary.
pub fn simple_guest_as_string() -> Result<String> {
    let buf = rust_guest_as_pathbuf("simpleguest");
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert simple guest PathBuf to string"))
}

/// Get a fully qualified OS-specific path to the dummyguest.exe
/// binary.
pub fn dummy_guest_as_string() -> Result<String> {
    let buf = rust_guest_as_pathbuf("dummyguest");
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert dummy guest PathBuf to string"))
}

/// Get a fully qualified OS-specific path to the callbackguest.exe
/// binary.
pub fn callback_guest_as_string() -> Result<String> {
    let buf = rust_guest_as_pathbuf("callbackguest");
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert callback guest PathBuf to string"))
}

pub fn c_guest_as_pathbuf(guest: &str) -> PathBuf {
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
            "Guests",
            guest,
            "x64",
            build_dir_selector,
            format!("{}.exe", guest).as_str(),
        ],
    )
}

pub fn c_simple_guest_as_string() -> Result<String> {
    let buf = c_guest_as_pathbuf("simpleguest");
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert simple guest PathBuf to string"))
}

pub fn c_callback_guest_as_string() -> Result<String> {
    let buf = c_guest_as_pathbuf("callbackguest");
    buf.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("couldn't convert callback guest PathBuf to string"))
}
