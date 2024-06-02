use std::path::PathBuf;

// uggh, so when publishing the cwd/manifest_dir is *not* the workspace root
fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let dev_candidate = manifest_dir.join("..").join("readme.md");
    let publish_candidate = manifest_dir.join("readme.md");

    for candidate in &[dev_candidate, publish_candidate] {
        if std::fs::metadata(candidate).is_ok() {
            println!("cargo:rustc-env=README_PATH={}", candidate.display());
            return;
        }
    }
}
