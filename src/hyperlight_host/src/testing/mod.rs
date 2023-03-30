use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;

pub(crate) const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

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

/// Get a new `PathBuf` pointing to `simpleguest.exe`
pub(crate) fn simple_guest_buf() -> PathBuf {
    join_to_path(MANIFEST_DIR, vec!["testdata", "simpleguest.exe"])
}

/// Get a new `PathBuf` pointing to `callbackguest.exe`
pub(crate) fn callback_guest_buf() -> PathBuf {
    join_to_path(MANIFEST_DIR, vec!["testdata", "callbackguest.exe"])
}

/// Read the file at `path_buf` into a `Vec<u8>` and return it,
/// or return `Err` if that went wrong
pub(crate) fn bytes_for_path(path_buf: PathBuf) -> Result<Vec<u8>> {
    let guest_path = path_buf
        .as_path()
        .to_str()
        .ok_or_else(|| anyhow!("couldn't convert guest {:?} to a path", path_buf))?;
    let guest_bytes = fs::read(guest_path)
        .map_err(|e| anyhow!("failed to open guest at path {guest_path} ({e})"))?;
    Ok(guest_bytes)
}
