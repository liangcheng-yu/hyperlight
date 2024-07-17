use std::fs::File;
use std::io::Write;
use std::path::Path;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "."]
#[include = "*.h"]
#[exclude = "target/*.h"]
struct Assets;

/// Extract_files extracts all of the header files embedded in this crate
/// to the specified folder preserving the folder structure.
/// We are doing this so we can consume these header files in other Rust projects.
pub fn extract_files(p: &Path) -> Result<(), String> {
    if !Path::new(p).exists() {
        return Err::<(), String>(format!("{} does not exist", p.display()));
    }

    for f in Assets::iter() {
        let embedded_path = f.as_ref();
        let full_path = p.join(embedded_path);
        let parent_dir = full_path.parent().unwrap();

        if !(parent_dir.exists()) {
            let _ = std::fs::create_dir_all(parent_dir);
        }
        println!("writing {}", parent_dir.display());

        let mut target = File::create(&full_path).unwrap();
        let embedded_file = Assets::get(embedded_path).unwrap();
        target.write_all(embedded_file.data.as_ref()).unwrap();
    }

    Ok(())
}
