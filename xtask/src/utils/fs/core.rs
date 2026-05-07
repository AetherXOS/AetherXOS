use anyhow::{Context, Result};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

/// Recursively copy a directory tree.
pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path: PathBuf = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), dest_path)?;
        }
    }

    Ok(())
}

/// Attempt to remove a file with multiple retries (useful on Windows).
pub fn try_remove_file_with_retries(path: &Path, retries: usize) -> Result<()> {
    for attempt in 0..=retries {
        match fs::remove_file(path) {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && attempt < retries => {
                thread::sleep(Duration::from_millis(250 * (attempt as u64 + 1)));
            }
            Err(e) => {
                return Err(e).with_context(|| {
                    format!(
                        "Failed deleting {} (attempt {})",
                        path.display(),
                        attempt + 1
                    )
                });
            }
        }
    }
    Ok(())
}
