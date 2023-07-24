use crate::hypervisor::surrogate_process_manager::SURROGATE_PROCESS_BINARY_NAME;
use std::path::Path;
use std::sync::Once;

pub(crate) fn copy_surrogate_exe() -> bool {
    static INIT: Once = Once::new();
    static mut RESULT: bool = false;

    #[cfg(debug_assertions)]
    let config = "Debug";
    #[cfg(not(debug_assertions))]
    let config = "Release";
    unsafe {
        INIT.call_once(|| {
            RESULT = {
                let test_binary = std::env::current_exe().unwrap();
                let dest_directory = test_binary.parent().unwrap();

                let source = dest_directory
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("src")
                    .join("HyperlightSurrogate")
                    .join("x64")
                    .join(config)
                    .join(SURROGATE_PROCESS_BINARY_NAME);

                let source_path = Path::new(&source);
                let dest_path = dest_directory.join(SURROGATE_PROCESS_BINARY_NAME);
                if source_path.exists() {
                    println!(
                        "Copying surrogate binary from {:?} to {:?}...",
                        source_path, dest_path
                    );
                    std::fs::copy(
                        source_path,
                        dest_directory.join(SURROGATE_PROCESS_BINARY_NAME),
                    )
                    .unwrap();
                    true
                } else {
                    println!("Surrogate binary not found at {:?}.", source_path);
                    false
                }
            }
        });
        RESULT
    }
}
