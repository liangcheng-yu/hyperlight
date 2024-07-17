use std::env;

use anyhow::{anyhow, Result};
use cbindgen::{Builder, Config};

fn main() -> Result<()> {
    let header_file_path = match env::var("GITHUB_WORKSPACE") {
        Ok(ws) => format!("{}/src/hyperlight_capi/include/hyperlight_capi.h", ws),
        Err(_) => "./include/hyperlight_capi.h".to_string(),
    };
    // re-run the build if either this script of the header file
    // is changed (or deleted!), even if the rust code is completely
    // unchanged.
    println!("cargo:rerun-if-changed=./src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=./include/hyperlight_capi.h");

    let crate_dir = env::var("CARGO_MANIFEST_DIR")?;
    let cfg = Config::from_file("cbindgen.toml").map_err(|e| anyhow!(e))?;
    Builder::new()
        .with_config(cfg)
        .with_crate(crate_dir)
        .generate()
        .expect("Unable to generate bingds for hyperlight_capi")
        .write_to_file(header_file_path);

    Ok(())
}
