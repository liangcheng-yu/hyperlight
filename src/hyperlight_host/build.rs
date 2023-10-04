use anyhow::{anyhow, Result};
use cbindgen::{Builder, Config};
use std::env;

fn main() -> Result<()> {
    let header_file_path = match env::var("GITHUB_WORKSPACE") {
        Ok(ws) => format!("{}/src/hyperlight_host/include/hyperlight_host.h", ws),
        Err(_) => "./include/hyperlight_host.h".to_string(),
    };
    // re-run the build if either this script or the header file
    // is changed (or deleted!), even if the rust code is completely
    // unchanged.
    println!("cargo:rerun-if-changed=./src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=./include/hyperlight_host.h");

    // Windows requires the HyperlightSurrogate.exe binary to be next to the executable running
    // hyperlight. We are using rust-ebmed to include the binary in the hyperlight_host library
    // and then extracting it at runtime why the surrogate process manager starts and needed pass
    // the location of the binary to the rust build.
    #[cfg(target_os = "windows")]
    {
        // Set $PROFILE env var during rust build so we can
        // use it with RustEmbed to specify which HyperlightSurrogate.exe
        // to include as an embedded resource in the surrograte_process_manager
        let profile = env::var("PROFILE")?;
        println!("cargo:rustc-env=PROFILE={}", profile)
    }

    let crate_dir = env::var("CARGO_MANIFEST_DIR")?;
    let cfg = Config::from_file("cbindgen.toml").map_err(|e| anyhow!(e))?;
    Builder::new()
        .with_config(cfg)
        .with_crate(crate_dir)
        .generate()
        .expect("Unable to generate FlatBuffers bindings for hyperlight_host")
        .write_to_file(header_file_path);

    Ok(())
}
