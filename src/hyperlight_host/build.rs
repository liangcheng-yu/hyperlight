use anyhow::Result;

fn main() -> Result<()> {
    // re-run the build if this script is changed (or deleted!),
    // even if the rust code is completely unchanged.
    println!("cargo:rerun-if-changed=*");

    // Windows requires the HyperlightSurrogate.exe binary to be next to the executable running
    // hyperlight. We are using rust-ebmed to include the binary in the hyperlight_host library
    // and then extracting it at runtime why the surrogate process manager starts and needed pass
    // the location of the binary to the rust build.
    #[cfg(target_os = "windows")]
    {
        // Set $HYPERLIGHT_SURROGATE_DIR env var during rust build so we can
        // use it with RustEmbed to specify where hyperlight_surrogate.exe is
        // to include as an embedded resource in the surrograte_process_manager
        let surrogate_bin_dep_path = std::env::var("CARGO_BIN_FILE_HYPERLIGHT_SURROGATE_HYPERLIGHT_SURROGATE")?;
        let surrogate_bin_dir = std::path::Path::new(&surrogate_bin_dep_path)
        .parent()
        .ok_or_else(|| anyhow::anyhow!("unable to find directory for hyperlight_surrogate.exe"))?;
        println!("cargo:rustc-env=HYPERLIGHT_SURROGATE_DIR={}", &surrogate_bin_dir.display());
    }

    Ok(())
}
