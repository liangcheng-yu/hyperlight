use anyhow::Result;

fn main() -> Result<()> {
    // re-run the build if this script is changed (or deleted!),
    // even if the rust code is completely unchanged.
    println!("cargo:rerun-if-changed=build.rs");

    // Windows requires the HyperlightSurrogate.exe binary to be next to the executable running
    // hyperlight. We are using rust-ebmed to include the binary in the hyperlight_host library
    // and then extracting it at runtime why the surrogate process manager starts and needed pass
    // the location of the binary to the rust build.
    #[cfg(target_os = "windows")]
    {
        // Set $PROFILE env var during rust build so we can
        // use it with RustEmbed to specify which HyperlightSurrogate.exe
        // to include as an embedded resource in the surrograte_process_manager
        let profile = std::env::var("PROFILE")?;
        println!("cargo:rustc-env=PROFILE={}", profile)
    }

    Ok(())
}
