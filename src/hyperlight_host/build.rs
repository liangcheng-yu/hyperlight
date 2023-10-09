use anyhow::Result;

fn main() -> Result<()> {
    // re-run the build if this script is changed (or deleted!),
    // even if the rust code is completely unchanged.
    println!("cargo:rerun-if-changed=*");

    // Windows requires the hyperlight_surrogate.exe binary to be next to the executable running
    // hyperlight. We are using rust-ebmed to include the binary in the hyperlight_host library
    // and then extracting it at runtime why the surrogate process manager starts and needed pass
    // the location of the binary to the rust build.
    #[cfg(target_os = "windows")]
    {
        // Build hyperlight_surrogate and
        // Set $HYPERLIGHT_SURROGATE_DIR env var during rust build so we can
        // use it with RustEmbed to specify where hyperlight_surrogate.exe is
        // to include as an embedded resource in the surrograte_process_manager

        // We need to copy/rename the source for hyperlight surrogate into a
        // temp directory because we cannot include a file name `Cargo.toml`
        // inside this package.
        let out_dir = std::env::var("OUT_DIR")?;
        std::fs::create_dir_all(format!("{out_dir}/hyperlight_surrogate/src"))?;
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        std::fs::copy(
            format!("{manifest_dir}/src/hyperlight_surrogate/src/main.rs"),
            format!("{out_dir}/hyperlight_surrogate/src/main.rs"),
        )?;
        std::fs::copy(
            format!("{manifest_dir}/src/hyperlight_surrogate/Cargo.toml_temp_name"),
            format!("{out_dir}/hyperlight_surrogate/Cargo.toml"),
        )?;
        let target_manifest_path = format!("{out_dir}/hyperlight_surrogate/Cargo.toml");

        // Note: When we build hyperlight_surrogate.exe CARGO_TARGET_DIR cannot
        // be the same as the CARGO_TARGET_DIR for the hyperlight_host otherwise
        // the build script will hang. Using a sub directory works tho!
        // xref - https://github.com/rust-lang/cargo/issues/6412
        let target_dir = std::path::PathBuf::from(&out_dir).join("..\\..\\hls");

        let profile = std::env::var("PROFILE")?;
        let _process = std::process::Command::new("cargo")
            .env("CARGO_TARGET_DIR", &target_dir)
            .arg("build")
            .arg("--manifest-path")
            .arg(&target_manifest_path)
            .arg("--profile")
            .arg(&profile)
            .arg("--verbose")
            .output()
            .unwrap();

        println!("cargo:rustc-env=PROFILE={}", profile);
        let surrogate_binary_dir = std::path::PathBuf::from(&target_dir).join(profile);

        println!(
            "cargo:rustc-env=HYPERLIGHT_SURROGATE_DIR={}",
            &surrogate_binary_dir.display()
        );
    }

    Ok(())
}
