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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_root_resolution() {
        let root = repo_root();
        assert!(root.is_absolute(), "Repository root must always resolve to an absolute host architecture path.");
        assert!(root.exists(), "Repository root path must physically exist during xtask operational lifetime.");
    }

    #[test]
    fn test_path_resolution_mechanics() {
        let relative = resolve("artifacts/boot_image");
        assert!(relative.is_absolute(), "Relative raw strings should be transformed into absolute workspace locators.");
        
        // Ensure absolute paths are wholly unaffected by mapping resolution
        #[cfg(unix)]
        let absolute_mock = "/tmp/xtask_mock_dir";
        #[cfg(windows)]
        let absolute_mock = "C:\\Windows\\Temp";
        
        let pass_through = resolve(absolute_mock);
        assert_eq!(pass_through.to_str().unwrap(), absolute_mock);
    }

    #[test]
    fn test_ensure_dir_lifecycle() {
        let test_dir = resolve("artifacts/xtask_test_validation_dir_9x");
        let _ = std::fs::remove_dir_all(&test_dir); // Ensure clean staging before tests
        
        assert!(ensure_dir(&test_dir).is_ok());
        assert!(test_dir.exists(), "Target directory must be accurately materialized successfully by xtask pathing logic.");
        
        // Execute manual cleanup post logic assertion
        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
