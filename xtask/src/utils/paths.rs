use std::path::{Path, PathBuf};

/// Returns the repository root directory (parent of the xtask crate).
pub fn repo_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .expect("xtask must be nested one level under repo root")
        .to_path_buf()
}

/// Resolve a path relative to the repo root; absolute paths are returned as-is.
pub fn resolve(relative: &str) -> PathBuf {
    let p = Path::new(relative);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        repo_root().join(p)
    }
}

/// Ensure a directory exists, creating it and all parents if necessary.
pub fn ensure_dir(dir: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}
